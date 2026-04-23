import { useEffect, useRef, useState, useCallback, memo } from 'react';

/**
 * Lightweight, non-interactive scroll position indicator for the chat
 * message area (antd Bubble.List).
 *
 * antd's Bubble.List uses `flex-direction: column-reverse` for auto-scroll,
 * which inverts the scroll coordinate system (scrollTop=0 = bottom of content).
 * OverlayScrollbars cannot handle this, so we render a simple visual-only
 * indicator that correctly accounts for the reversed layout.
 *
 * The indicator auto-shows on scroll and fades out after inactivity.
 */
function ChatScrollIndicatorInner() {
  const [thumb, setThumb] = useState({ top: 0, height: 0, opacity: 0 });
  const hideTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const elRef = useRef<HTMLElement | null>(null);

  const handleScroll = useCallback(() => {
    const el = elRef.current;
    if (!el) return;

    const { scrollTop, scrollHeight, clientHeight } = el;
    if (scrollHeight <= clientHeight + 1) {
      setThumb((s) => (s.opacity === 0 ? s : { ...s, opacity: 0 }));
      return;
    }

    const maxScroll = scrollHeight - clientHeight;
    const ratio = clientHeight / scrollHeight;
    const thumbH = Math.max(ratio * clientHeight, 24);

    const isReversed = getComputedStyle(el).flexDirection === 'column-reverse';

    // Normalise to 0 = top-of-content, 1 = bottom-of-content.
    // In column-reverse scrollTop is 0 at the bottom and goes negative upward.
    let progress: number;
    if (isReversed) {
      progress = 1 + scrollTop / maxScroll;
    } else {
      progress = scrollTop / maxScroll;
    }
    progress = Math.max(0, Math.min(1, progress));

    const top = progress * (clientHeight - thumbH);
    setThumb({ top, height: thumbH, opacity: 1 });

    clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      setThumb((s) => ({ ...s, opacity: 0 }));
    }, 1200);
  }, []);

  useEffect(() => {
    const attach = () => {
      const el = document.querySelector<HTMLElement>('.ant-bubble-list-scroll-box');
      if (!el || el === elRef.current) return;
      elRef.current?.removeEventListener('scroll', handleScroll);
      elRef.current = el;
      el.addEventListener('scroll', handleScroll, { passive: true });
      // Calculate initial position immediately (Bubble.List's auto-scroll
      // may have already fired before our listener was attached)
      handleScroll();
    };

    // Defer initial attach to allow Bubble.List to mount
    const raf = requestAnimationFrame(attach);

    // Re-attach when DOM changes (route switch, conversation change)
    const observer = new MutationObserver(attach);
    observer.observe(document.body, { childList: true, subtree: true });

    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
      elRef.current?.removeEventListener('scroll', handleScroll);
      clearTimeout(hideTimer.current);
    };
  }, [handleScroll]);

  return (
    <div
      className="chat-scroll-indicator"
      style={{
        position: 'absolute',
        right: 2,
        top: thumb.top,
        width: 5,
        height: thumb.height,
        borderRadius: 3,
        opacity: thumb.opacity,
        transition: 'opacity 0.3s ease',
        pointerEvents: 'none',
        zIndex: 1,
      }}
    />
  );
}

export const ChatScrollIndicator = memo(ChatScrollIndicatorInner);
