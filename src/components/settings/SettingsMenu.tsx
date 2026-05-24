import { type ReactNode } from "react";

export interface SettingsMenuItemType {
  key: string;
  label: ReactNode;
  icon?: ReactNode;
}

export interface SettingsMenuGroupType {
  type: "group";
  label: string;
  children: SettingsMenuItemType[];
}

export type SettingsMenuItem = SettingsMenuItemType | SettingsMenuGroupType;

interface SettingsMenuProps {
  items: SettingsMenuItem[];
  selectedKeys?: string[];
  onClick?: (info: { key: string }) => void;
}

function isGroup(item: SettingsMenuItem): item is SettingsMenuGroupType {
  return "type" in item && item.type === "group";
}

export function SettingsMenu({ items, selectedKeys, onClick }: SettingsMenuProps) {
  const selectedKey = selectedKeys?.[0] ?? "";

  return (
    <nav className="settings-menu">
      {items.map((item) => {
        if (isGroup(item)) {
          return (
            <div key={item.label} className="settings-menu-group">
              <div className="settings-menu-group-label">{item.label}</div>
              {item.children.map((child) => (
                <button
                  key={child.key}
                  className={`settings-menu-item${selectedKey === child.key ? " active" : ""}`}
                  onClick={() => onClick?.({ key: child.key })}
                >
                  {child.icon && <span className="settings-menu-item-icon">{child.icon}</span>}
                  <span className="settings-menu-item-label">{child.label}</span>
                </button>
              ))}
            </div>
          );
        }
        return (
          <button
            key={item.key}
            className={`settings-menu-item${selectedKey === item.key ? " active" : ""}`}
            onClick={() => onClick?.({ key: item.key })}
          >
            {item.icon && <span className="settings-menu-item-icon">{item.icon}</span>}
            <span className="settings-menu-item-label">{item.label}</span>
          </button>
        );
      })}
    </nav>
  );
}
