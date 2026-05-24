import {
  cloneElement,
  isValidElement,
  type ReactElement,
  type ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";

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
}

export function Tooltip(
  { title, children, placement = "top", mouseEnterDelay = 0.3, open: controlledOpen }: TooltipProps,
) {
  const [internalVisible, setInternalVisible] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const isControlled = controlledOpen !== undefined;
  const visible = isControlled ? controlledOpen : internalVisible;

  const show = useCallback(() => {
    if (isControlled) { return; }
    timeoutRef.current = setTimeout(() => setInternalVisible(true), mouseEnterDelay * 1000);
  }, [mouseEnterDelay, isControlled]);

  const hide = useCallback(() => {
    if (isControlled) { return; }
    if (timeoutRef.current) { clearTimeout(timeoutRef.current); }
    setInternalVisible(false);
  }, [isControlled]);

  useEffect(() => () => {
    if (timeoutRef.current) { clearTimeout(timeoutRef.current); }
  }, []);

  const child = isValidElement(children) ? children : null;
  if (!child) { return null; }

  return (
    <>
      {cloneElement(
        child as ReactElement<
          { ref?: unknown; onMouseEnter?: unknown; onMouseLeave?: unknown; onFocus?: unknown; onBlur?: unknown }
        >,
        {
          ref: (el: HTMLElement | null) => {
            void el;
          },
          onMouseEnter: show,
          onMouseLeave: hide,
          onFocus: show,
          onBlur: hide,
        },
      )}
      {visible && (
        <div className={`tooltip-content tooltip-${typeof placement === "string" ? placement : "top"}`}>
          {title}
        </div>
      )}
    </>
  );
}
