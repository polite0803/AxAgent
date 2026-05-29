import { type ReactElement, type ReactNode, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

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
  const panelRef = useRef<HTMLDivElement>(null);
  const [panelStyle, setPanelStyle] = useState<React.CSSProperties>({});

  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : internalOpen;

  const setOpen = useCallback((v: boolean) => {
    if (!isControlled) { setInternalOpen(v); }
    onOpenChange?.(v);
  }, [isControlled, onOpenChange]);

  const close = useCallback(() => setOpen(false), [setOpen]);

  const isClickTrigger = !trigger || trigger.includes("click");

  const updatePosition = useCallback(() => {
    if (!containerRef.current || !panelRef.current) { return; }
    const tr = containerRef.current.getBoundingClientRect();
    const ph = panelRef.current.offsetHeight;
    const pw = panelRef.current.offsetWidth;

    // 紧贴触发按钮：下方 2px 间距，左对齐
    let top = tr.bottom + 2;
    let left = tr.left;

    // 超出底部 → 翻转到上方
    if (top + ph > window.innerHeight - 8) { top = tr.top - ph - 2; }
    // 顶部也溢出 → 贴顶
    if (top < 8) { top = 8; }

    // 右侧溢出 → 右对齐
    if (left + pw > window.innerWidth - 8) { left = window.innerWidth - pw - 8; }
    // 左侧溢出 → 贴左
    if (left < 8) { left = 8; }

    setPanelStyle({ position: "fixed", top, left, zIndex: 9999 });
  }, []);

  useLayoutEffect(() => {
    if (!open) { return; }
    const raf = requestAnimationFrame(updatePosition);
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [open, updatePosition]);

  useEffect(() => {
    if (!open) { return; }
    const handler = (e: MouseEvent) => {
      const t = e.target as Node;
      if (!containerRef.current?.contains(t) && !panelRef.current?.contains(t)) { close(); }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open, close]);

  const panel = open && (
    <div ref={panelRef} className="dropdown-panel" role="menu" style={panelStyle}>
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
  );

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
      {panel && createPortal(panel, document.body)}
    </div>
  );
}
