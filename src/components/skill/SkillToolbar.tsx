import { executeActionChain } from "@/lib/skillActionExecutor";
import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore } from "@/stores";
import type { MergedToolbarButton } from "@/stores/feature/skillExtensionStore";
import { Button, Dropdown, Space, Tooltip } from "antd";
import { useCallback } from "react";
import { useNavigate } from "react-router-dom";

interface SkillToolbarProps {
  position: "left" | "right";
}

export function SkillToolbar({ position }: SkillToolbarProps) {
  const toolbarButtons = useSkillExtensionStore((s) => s.toolbarButtons);
  const navigate = useNavigate();

  const buttons = toolbarButtons
    .filter((b) => b.position === position)
    .sort((a, b) => a.priority - b.priority);

  if (buttons.length === 0) { return null; }

  return (
    <Space size={2}>
      {buttons.map((btn) => <ToolbarButton key={`${btn.skillName}:${btn.id}`} button={btn} navigate={navigate} />)}
    </Space>
  );
}

function ToolbarButton({ button, navigate }: { button: MergedToolbarButton; navigate: (path: string) => void }) {
  const IconComp = resolveIconComponent(button.icon);

  const handleClick = useCallback(() => {
    executeActionChain(button.onClick, navigate);
  }, [button.onClick, navigate]);

  const buttonEl = (
    <Tooltip title={button.tooltip}>
      <Button type="text" size="small" icon={<IconComp size={14} />} onClick={handleClick} />
    </Tooltip>
  );

  if (button.menu && button.menu.length > 0) {
    const menuItems = button.menu.map((item, i) => ({
      key: String(i),
      label: item.label,
      onClick: () => executeActionChain(item.actions, navigate),
    }));

    return (
      <Dropdown menu={{ items: menuItems }} trigger={["click"]}>
        {buttonEl}
      </Dropdown>
    );
  }

  return buttonEl;
}
