import { Button, Dropdown, Tag } from "antd";
import type { MenuProps } from "antd";
import { Container, Monitor, Plus, Terminal } from "lucide-react";
import { useTranslation } from "react-i18next";

const backendIcons: Record<string, React.ReactNode> = {
  local: <Monitor size={14} />,
  docker: <Container size={14} />,
  ssh: <Terminal size={14} />,
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
  const { t } = useTranslation();
  const items: MenuProps["items"] = backends.map((b) => ({
    key: b.type,
    icon: backendIcons[b.type],
    label: (
      <div className="flex items-center justify-between gap-4" style={{ minWidth: 180 }}>
        <span>{t(`terminal.${b.type}`)}</span>
        <Tag color={b.connected ? "green" : "default"} style={{ margin: 0 }}>
          {b.connected ? t("terminal.sessions", { count: b.sessions }) : t("terminal.offline")}
        </Tag>
      </div>
    ),
    onClick: () => onSelect(b.type),
  }));

  items.push({ type: "divider" });
  items.push({
    key: "configure",
    icon: <Plus size={14} />,
    label: t("terminal.configureBackends"),
    onClick: () => onConfigure(current),
  });

  return (
    <Dropdown menu={{ items }} trigger={["click"]}>
      <Button size="small" icon={backendIcons[current]}>
        {t(`terminal.${current}`)}
      </Button>
    </Dropdown>
  );
}
