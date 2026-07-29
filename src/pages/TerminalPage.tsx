// SPDX-License-Identifier: AGPL-3.0-only

import { DockerConfigModal } from "@/components/terminal/DockerConfigModal";
import { IntegratedTerminal } from "@/components/terminal/IntegratedTerminal";
import { SshConfigModal } from "@/components/terminal/SshConfigModal";
import { StatusBarWidget } from "@/components/terminal/StatusBarWidget";
import { TerminalBackendSelector } from "@/components/terminal/TerminalBackendSelector";
import { message } from "@/lib/toast";
import { useTerminalStore } from "@/stores/feature/terminalStore";
import { Tabs } from "antd";
import { Folder, SquareTerminal } from "lucide-react";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { FilesPage } from "./FilesPage";

/**
 * 终端页面：合并了「终端」与「文件」两个 Tab。
 * 文件原为独立侧栏导航项，现作为终端页内的二级 Tab，减少导航层级。
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

  const terminalTab = (
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

  const tabItems = [
    {
      key: "terminal",
      label: (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
          <SquareTerminal size={14} /> {t("nav.terminal")}
        </span>
      ),
      children: terminalTab,
    },
    {
      key: "files",
      label: (
        <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
          <Folder size={14} /> {t("nav.files")}
        </span>
      ),
      children: <FilesPage />,
    },
  ];

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <Tabs
        defaultActiveKey="terminal"
        items={tabItems}
        className="ax-fill-tabs"
        style={{ padding: "0 16px" }}
        tabBarStyle={{ flexShrink: 0, marginBottom: 0 }}
        destroyInactiveTabPane
      />
    </div>
  );
}
