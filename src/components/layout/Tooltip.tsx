// SPDX-License-Identifier: AGPL-3.0-only

import {
  cloneElement,
  forwardRef,
  isValidElement,
  type ReactElement,
  type ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";

const GAP = 8;

/** 根据 placement 计算 tooltip 的样式（fixed 定位 + transform） */
function computeTooltipStyle(
  rect: DOMRect,
  placement: string,
): React.CSSProperties {
  const base: React.CSSProperties = {
    position: "fixed",
  };

  switch (placement) {
    case "top":
      base.left = rect.left + rect.width / 2;
      base.top = rect.top - GAP;
      base.transform = "translateX(-50%)";
      break;
    case "bottom":
      base.left = rect.left + rect.width / 2;
      base.top = rect.bottom + GAP;
      base.transform = "translateX(-50%)";
      break;
    case "left":
      base.left = rect.left - GAP;
      base.top = rect.top + rect.height / 2;
      base.transform = "translateX(-100%) translateY(-50%)";
      break;
    case "right":
      base.left = rect.right + GAP;
      base.top = rect.top + rect.height / 2;
      base.transform = "translateY(-50%)";
      break;
    default: // fallback: top
      base.left = rect.left + rect.width / 2;
      base.top = rect.top - GAP;
      base.transform = "translateX(-50%)";
  }

  return base;
}

interface TooltipProps {
  title: ReactNode;
  children: ReactElement;
  placement?: "top" | "bottom" | "left" | "right" | string;
  mouseEnterDelay?: number;
  open?: boolean;
  // antd 兼容属性
  overlayStyle?: React.CSSProperties;
  color?: string;
  arrow?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  // 透传给子元素的事件处理器（Popconfirm/Popover 等通过 cloneElement
  // 把 onClick 注入到 Tooltip，但 Tooltip 不再把 onClick 转给真实 DOM，
  // 会导致 Popconfirm/Tooltip → span 的嵌套点击没反应）
  onClick?: React.MouseEventHandler<HTMLElement>;
  onKeyDown?: React.KeyboardEventHandler<HTMLElement>;
  onKeyUp?: React.KeyboardEventHandler<HTMLElement>;
  onContextMenu?: React.MouseEventHandler<HTMLElement>;
}

export const Tooltip = forwardRef<HTMLElement, TooltipProps>(
  (
    {
      title,
      children,
      placement = "top",
      mouseEnterDelay = 0.3,
      open: controlledOpen,
      onClick,
      onKeyDown,
      onKeyUp,
      onContextMenu,
    },
    forwardedRef,
  ) => {
    const [internalVisible, setInternalVisible] = useState(false);
    const [tooltipStyle, setTooltipStyle] = useState<React.CSSProperties>({});
    const triggerRef = useRef<HTMLElement | null>(null);
    const timeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const mountedRef = useRef(true);

    const isControlled = controlledOpen !== undefined;
    const visible = isControlled ? controlledOpen : internalVisible;

    const show = useCallback(() => {
      if (isControlled) { return; }
      timeoutRef.current = setTimeout(() => {
        if (mountedRef.current) {
          setInternalVisible(true);
        }
      }, mouseEnterDelay * 1000);
    }, [mouseEnterDelay, isControlled]);

    const hide = useCallback(() => {
      if (isControlled) { return; }
      if (timeoutRef.current) { clearTimeout(timeoutRef.current); }
      if (mountedRef.current) {
        setInternalVisible(false);
      }
    }, [isControlled]);

    // 清理 timeout
    useEffect(() => {
      mountedRef.current = true;
      return () => {
        mountedRef.current = false;
        if (timeoutRef.current) { clearTimeout(timeoutRef.current); }
      };
    }, []);

    // 每次展示时重新计算位置
    useEffect(() => {
      if (visible && triggerRef.current) {
        const rect = triggerRef.current.getBoundingClientRect();
        setTooltipStyle(computeTooltipStyle(rect, placement));
      }
    }, [visible, placement]);

    const child = isValidElement(children) ? children : null;
    if (!child) { return null; }

    return (
      <>
        {cloneElement(
          child as ReactElement<
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            any
          >,
          {
            ref: (el: HTMLElement | null) => {
              triggerRef.current = el;
              if (typeof forwardedRef === "function") {
                forwardedRef(el);
              } else if (forwardedRef && "current" in forwardedRef) {
                (forwardedRef as React.MutableRefObject<HTMLElement | null>).current = el;
              }
            },
            onMouseEnter: show,
            onMouseLeave: hide,
            onFocus: show,
            onBlur: hide,
            // 透传父组件注入的事件，让 Popconfirm/Popover 等
            // 通过 cloneElement 传入的 onClick 能真正触发到子元素
            onClick,
            onKeyDown,
            onKeyUp,
            onContextMenu,
          },
        )}
        {visible && (
          <div className="tooltip-content" style={tooltipStyle}>
            {title}
          </div>
        )}
      </>
    );
  },
);
