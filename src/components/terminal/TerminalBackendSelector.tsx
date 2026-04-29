import { Button, Dropdown, Tag } from "antd";
import { Monitor, Container, Terminal, Plus } from "lucide-react";
import type { MenuProps } from "antd";

const backendIcons: Record<string, React.ReactNode> = {
  local: <Monitor size={14} />,
  docker: <Container size={14} />,
  ssh: <Terminal size={14} />,
};

const backendLabels: Record<string, string> = {
  local: "Local",
  docker: "Docker",
  ssh: "SSH",
};

interface TerminalBackendSelectorProps {
  current: string;
  backends: Array<{ type: string; connected: boolean; sessions: number }>;
  onSelect: (backendType: string) => void;
  onConfigure: (backendType: string) => void;
}

export function TerminalBackendSelector({
  current,
  backends,
  onSelect,
  onConfigure,
}: TerminalBackendSelectorProps) {
  const items: MenuProps["items"] = backends.map((b) => ({
    key: b.type,
    icon: backendIcons[b.type],
    label: (
      <div className="flex items-center justify-between gap-4" style={{ minWidth: 180 }}>
        <span>{backendLabels[b.type] ?? b.type}</span>
        <Tag color={b.connected ? "green" : "default"} style={{ margin: 0 }}>
          {b.connected ? `${b.sessions} sessions` : "Offline"}
        </Tag>
      </div>
    ),
    onClick: () => onSelect(b.type),
  }));

  items.push({ type: "divider" });
  items.push({
    key: "configure",
    icon: <Plus size={14} />,
    label: "Configure Backends...",
    onClick: () => onConfigure(current),
  });

  return (
    <Dropdown menu={{ items }} trigger={["click"]}>
      <Button size="small" icon={backendIcons[current]}>
        {backendLabels[current] ?? current}
      </Button>
    </Dropdown>
  );
}
