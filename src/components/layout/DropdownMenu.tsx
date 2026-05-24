import { type ReactElement, type ReactNode, useCallback, useEffect, useRef, useState } from "react";

export interface DropdownItem {
  key: string;
  label?: ReactNode;
  icon?: ReactNode;
  onClick?: () => void;
  danger?: boolean;
  divider?: boolean;
  disabled?: boolean;
  children?: DropdownItem[];
  type?: "group";
}

export function toDropdownItems<T extends { key: string }>(
  items: T[],
  onClick: (key: string) => void,
): DropdownItem[] {
  return items.map((item) => ({ ...item, onClick: () => onClick(item.key) }));
}

interface DropdownMenuProps {
  items: DropdownItem[];
  trigger?: ("click" | "hover" | "contextMenu")[];
  children: ReactElement;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

function DropdownSubmenu({ item, close }: { item: DropdownItem; close: () => void }) {
  const [subOpen, setSubOpen] = useState(false);

  return (
    <div
      className="dropdown-submenu"
      onMouseEnter={() => setSubOpen(true)}
      onMouseLeave={() => setSubOpen(false)}
    >
      <button
        className="dropdown-item dropdown-item-has-children"
        role="menuitem"
      >
        {item.icon && <span className="dropdown-item-icon">{item.icon}</span>}
        <span className="dropdown-item-label">{item.label}</span>
        <span className="dropdown-item-arrow">▸</span>
      </button>
      {subOpen && item.children && (
        <div className="dropdown-submenu-panel">
          {item.children.map((child) =>
            child.divider
              ? <div key={child.key} className="dropdown-divider" />
              : child.children
              ? <DropdownSubmenu key={child.key} item={child} close={close} />
              : (
                <button
                  key={child.key}
                  className={`dropdown-item${child.danger ? " dropdown-item-danger" : ""}${
                    child.disabled ? " dropdown-item-disabled" : ""
                  }`}
                  role="menuitem"
                  disabled={child.disabled}
                  onClick={() => {
                    if (!child.disabled) {
                      child.onClick?.();
                      close();
                    }
                  }}
                >
                  {child.icon && <span className="dropdown-item-icon">{child.icon}</span>}
                  <span className="dropdown-item-label">{child.label}</span>
                </button>
              )
          )}
        </div>
      )}
    </div>
  );
}

export function DropdownMenu(
  { items, children, trigger, open: controlledOpen, onOpenChange }: DropdownMenuProps,
) {
  const [internalOpen, setInternalOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : internalOpen;

  const setOpen = useCallback((v: boolean) => {
    if (!isControlled) { setInternalOpen(v); }
    onOpenChange?.(v);
  }, [isControlled, onOpenChange]);

  const close = useCallback(() => setOpen(false), [setOpen]);

  const isClickTrigger = !trigger || trigger.includes("click");

  useEffect(() => {
    if (!open) { return; }
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        close();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open, close]);

  return (
    <div
      ref={containerRef}
      className="dropdown-trigger"
      onClick={isClickTrigger ? () => setOpen(!open) : undefined}
      onContextMenu={trigger?.includes("contextMenu")
        ? (e) => {
          e.preventDefault();
          setOpen(!open);
        }
        : undefined}
      onKeyDown={(e) => {
        if (e.key === "Escape") { close(); }
      }}
    >
      {children}
      {open && (
        <div className="dropdown-panel" role="menu">
          {items.map((item) => {
            if (item.divider) { return <div key={item.key} className="dropdown-divider" />; }
            if (item.type === "group" && item.children) {
              return (
                <div key={item.key} className="dropdown-group">
                  {item.label && <div className="dropdown-group-label">{item.label}</div>}
                  {item.children.map((child) =>
                    child.divider
                      ? <div key={child.key} className="dropdown-divider" />
                      : (
                        <button
                          key={child.key}
                          className={`dropdown-item${child.danger ? " dropdown-item-danger" : ""}${
                            child.disabled ? " dropdown-item-disabled" : ""
                          }`}
                          role="menuitem"
                          disabled={child.disabled}
                          onClick={() => {
                            child.onClick?.();
                            close();
                          }}
                        >
                          {child.icon && <span className="dropdown-item-icon">{child.icon}</span>}
                          <span className="dropdown-item-label">{child.label}</span>
                        </button>
                      )
                  )}
                </div>
              );
            }
            if (item.children) { return <DropdownSubmenu key={item.key} item={item} close={close} />; }
            return (
              <button
                key={item.key}
                className={`dropdown-item${item.danger ? " dropdown-item-danger" : ""}${
                  item.disabled ? " dropdown-item-disabled" : ""
                }`}
                role="menuitem"
                disabled={item.disabled}
                onClick={() => {
                  if (!item.disabled) {
                    item.onClick?.();
                    close();
                  }
                }}
              >
                {item.icon && <span className="dropdown-item-icon">{item.icon}</span>}
                <span className="dropdown-item-label">{item.label}</span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
