import { DropdownMenu } from "@/components/layout/DropdownMenu";
import { Tooltip } from "@/components/layout/Tooltip";
import { executeActionChain } from "@/lib/skillActionExecutor";
import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore } from "@/stores";
import type { MergedToolbarButton } from "@/stores/feature/skillExtensionStore";
import { Button, Space } from "antd";
import { useCallback } from "react";
import { useNavigate } from "react-router-dom";

export function SkillToolbar() {
  const toolbarButtons = useSkillExtensionStore((s) => s.toolbarButtons);
  const navigate = useNavigate();

  const buttons = [...toolbarButtons].sort((a, b) => a.priority - b.priority);

  if (buttons.length === 0) {
    return null;
  }

  // 按 skillName 分组，组间添加分隔符
  const groups = new Map<string, typeof buttons>();
  for (const btn of buttons) {
    if (!groups.has(btn.skillName)) {
      groups.set(btn.skillName, []);
    }
    groups.get(btn.skillName)!.push(btn);
  }

  return (
    <Space size={2}>
      {Array.from(groups.entries()).map(([skillName, btns], groupIdx) => (
        <span
          key={skillName}
          style={{ display: "inline-flex", alignItems: "center", gap: 2 }}
        >
          {groupIdx > 0 && (
            <span
              style={{
                width: 1,
                height: 16,
                backgroundColor: "var(--color-border-secondary)",
                margin: "0 4px",
              }}
            />
          )}
          {btns.map((btn) => (
            <ToolbarButton
              key={`${btn.skillName}:${btn.id}`}
              button={btn}
              navigate={navigate}
            />
          ))}
        </span>
      ))}
    </Space>
  );
}

function ToolbarButton({
  button,
  navigate,
}: {
  button: MergedToolbarButton;
  navigate: (path: string) => void;
}) {
  const IconComp = resolveIconComponent(button.icon);

  const handleClick = useCallback(() => {
    executeActionChain(button.onClick, navigate);
  }, [button.onClick, navigate]);

  const buttonEl = (
    <Tooltip title={button.tooltip}>
      <Button
        type="text"
        size="small"
        icon={
          // eslint-disable-next-line react-hooks/static-components
          <IconComp size={14} />
        }
        onClick={handleClick}
      />
    </Tooltip>
  );

  if (button.menu && button.menu.length > 0) {
    const menuItems = button.menu.map((item, i) => ({
      key: String(i),
      label: item.label,
      onClick: () => executeActionChain(item.actions, navigate),
    }));

    return (
      <DropdownMenu items={menuItems} trigger={["click"]}>
        {buttonEl}
      </DropdownMenu>
    );
  }

  return buttonEl;
}
