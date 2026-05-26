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

  // 计算面板位置（Portal 到 body 后使用 position: fixed）
  const updatePosition = useCallback(() => {
    if (!containerRef.current || !panelRef.current) { return; }
    const triggerRect = containerRef.current.getBoundingClientRect();
    const panelH = panelRef.current.offsetHeight;
    const panelW = panelRef.current.offsetWidth;
    const viewportH = window.innerHeight;
    const viewportW = window.innerWidth;

    // 默认：面板右下角对齐触发器右上角（避免覆盖触发器左侧的其他按钮）
    let top = triggerRect.bottom + 2;
    let left = triggerRect.right - panelW;

    // 左侧越界 → 改为左对齐
    if (left < 8) {
      left = triggerRect.left;
    }
    // 还是越界 → 贴左
    if (left < 8) { left = 8; }
    // 右侧越界 → 贴右
    if (left + panelW > viewportW - 8) {
      left = viewportW - panelW - 8;
    }
    // 底部空间不足 → 翻转到上方
    if (top + panelH > viewportH - 8) {
      top = triggerRect.top - panelH - 2;
    }
    // 上方也不够 → 贴顶
    if (top < 8) { top = 8; }

    setPanelStyle({
      position: "fixed",
      top,
      left,
      zIndex: 9999,
    });
  }, []);

  // 面板打开时计算位置 + 监听 resize/scroll
  useLayoutEffect(() => {
    if (!open) { return; }
    // 等一帧让 panelRef 挂载完成，offsetHeight 有效
    const raf = requestAnimationFrame(updatePosition);
    window.addEventListener("resize", updatePosition);
    window.addEventListener("scroll", updatePosition, true);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", updatePosition);
      window.removeEventListener("scroll", updatePosition, true);
    };
  }, [open, updatePosition]);

  // 点击外部关闭（需同时检查触发器和面板，因面板已脱离触发器 DOM 子树）
  useEffect(() => {
    if (!open) { return; }
    const handler = (e: MouseEvent) => {
      const target = e.target as Node;
      const insideTrigger = containerRef.current?.contains(target);
      const insidePanel = panelRef.current?.contains(target);
      if (!insideTrigger && !insidePanel) {
        close();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open, close]);

  const renderPanel = () => (
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
      {open && createPortal(renderPanel(), document.body)}
    </div>
  );
}
