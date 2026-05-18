import React, { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

import {
  CHAT_SCROLL_IS_REVERSED,
  getDistanceToHistoryTop,
  getScrollTopAfterPrepend,
  hasScrollLayoutMetricsChanged,
  shouldIgnoreScrollDepartureFromBottom,
  shouldKeepAutoScroll,
  shouldShowScrollToBottom,
  shouldStickToBottomOnLayoutChange,
} from "./chatScroll";

export interface UseChatViewScrollParams {
  bubbleListRef: React.RefObject<any | null>;
  activeConversationId: string | null;
  bubbleListThemeKey: string;
  messageCount: number;
  streaming: boolean;
  hasOlderMessages: boolean;
  loading: boolean;
  loadingOlder: boolean;
  loadOlderMessages: () => Promise<void>;
  allBubbleItems: Array<{ key: string | number }>;
  lastBubbleKey: string;
}

export interface UseChatViewScrollReturn {
  showScrollToBottom: boolean;
  stickToBottom: boolean;
  scrollBoxRef: React.RefObject<HTMLElement | null>;
  handleBubbleListScroll: (event: React.UIEvent<HTMLDivElement>) => void;
  handleScrollToBottom: () => void;
  minimapScrollTo: (messageId: string) => void;
  syncScrollToBottomVisibility: () => void;
  markUserScrollIntent: () => void;
  pendingScrollConversationIdRef: React.RefObject<string | null>;
}

export function useChatViewScroll({
  bubbleListRef,
  activeConversationId,
  bubbleListThemeKey,
  messageCount,
  streaming,
  hasOlderMessages,
  loading,
  loadingOlder,
  loadOlderMessages,
  allBubbleItems,
  lastBubbleKey,
}: UseChatViewScrollParams): UseChatViewScrollReturn {
  const [showScrollToBottom, setShowScrollToBottom] = useState(false);
  const [stickToBottom, setStickToBottom] = useState(true);

  const scrollBoxRef = useRef<HTMLElement | null>(null);
  const scrollContentRef = useRef<HTMLElement | null>(null);
  const pendingScrollConversationIdRef = useRef<string | null>(activeConversationId ?? null);
  const stickToBottomRef = useRef(stickToBottom);
  const scrollLayoutMetricsRef = useRef({ scrollHeight: 0, clientHeight: 0 });
  const lastUserScrollIntentAtRef = useRef(0);

  const markUserScrollIntent = useCallback(() => {
    lastUserScrollIntentAtRef.current = Date.now();
  }, []);

  useLayoutEffect(() => {
    scrollBoxRef.current = (bubbleListRef.current?.scrollBoxNativeElement as HTMLElement) ?? null;
    scrollContentRef.current = (scrollBoxRef.current?.firstElementChild as HTMLElement | null) ?? null;
  });

  useEffect(() => {
    stickToBottomRef.current = stickToBottom;
  }, [stickToBottom]);

  useEffect(() => {
    const scrollBox = scrollBoxRef.current;
    if (!scrollBox) { return; }

    const handleUserIntent = () => {
      markUserScrollIntent();
    };

    scrollBox.addEventListener("wheel", handleUserIntent, { passive: true });
    scrollBox.addEventListener("touchstart", handleUserIntent, { passive: true });
    scrollBox.addEventListener("touchmove", handleUserIntent, { passive: true });
    scrollBox.addEventListener("pointerdown", handleUserIntent, { passive: true });

    return () => {
      scrollBox.removeEventListener("wheel", handleUserIntent);
      scrollBox.removeEventListener("touchstart", handleUserIntent);
      scrollBox.removeEventListener("touchmove", handleUserIntent);
      scrollBox.removeEventListener("pointerdown", handleUserIntent);
    };
  }, [activeConversationId, bubbleListThemeKey, markUserScrollIntent, messageCount]);

  const minimapScrollTo = useCallback((messageId: string) => {
    let scrollBox = scrollBoxRef.current;
    if (!scrollBox) {
      scrollBox = (bubbleListRef.current?.scrollBoxNativeElement as HTMLElement)
        ?? document.querySelector<HTMLElement>(".ant-bubble-list-scroll-box");
      if (scrollBox) { scrollBoxRef.current = scrollBox; }
    }
    if (!scrollBox) { return; }
    const marker = scrollBox.querySelector(`[data-axagent-msg="${messageId}"]`);
    if (!marker) { return; }
    let el: Element = marker;
    for (;;) {
      const parent = el.parentElement;
      if (!parent || parent === scrollBox) { break; }
      if (parent.parentElement === scrollBox) { break; }
      el = parent;
    }
    el.scrollIntoView({ behavior: "smooth", block: "start" });
  }, []);

  useEffect(() => {
    pendingScrollConversationIdRef.current = activeConversationId ?? null;
    setShowScrollToBottom(false);
    setStickToBottom(true);
    scrollLayoutMetricsRef.current = { scrollHeight: 0, clientHeight: 0 };
  }, [activeConversationId]);

  const syncScrollToBottomVisibility = useCallback(() => {
    const target = scrollBoxRef.current;
    if (!target) { return; }
    const nextShowScrollToBottom = shouldShowScrollToBottom(
      target.scrollHeight,
      target.scrollTop,
      target.clientHeight,
      CHAT_SCROLL_IS_REVERSED,
    );
    setShowScrollToBottom((prev) => (prev === nextShowScrollToBottom ? prev : nextShowScrollToBottom));
  }, []);

  const handleLoadOlderMessages = useCallback(async () => {
    const scrollContainer = bubbleListRef.current?.scrollBoxNativeElement as HTMLDivElement | null | undefined;
    const previousScrollHeight = scrollContainer?.scrollHeight ?? 0;
    const previousScrollTop = scrollContainer?.scrollTop ?? 0;
    await loadOlderMessages();
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        if (!scrollContainer) { return; }
        scrollContainer.scrollTop = getScrollTopAfterPrepend(
          previousScrollTop,
          previousScrollHeight,
          scrollContainer.scrollHeight,
          CHAT_SCROLL_IS_REVERSED,
        );
      });
    });
  }, [loadOlderMessages]);

  const handleBubbleListScroll = useCallback((event: React.UIEvent<HTMLDivElement>) => {
    const target = event.currentTarget;
    setShowScrollToBottom(
      shouldShowScrollToBottom(
        target.scrollHeight,
        target.scrollTop,
        target.clientHeight,
        CHAT_SCROLL_IS_REVERSED,
      ),
    );
    const keepAutoScroll = shouldKeepAutoScroll(
      target.scrollHeight,
      target.scrollTop,
      target.clientHeight,
      CHAT_SCROLL_IS_REVERSED,
      1,
    );
    const hadRecentUserScrollIntent = Date.now() - lastUserScrollIntentAtRef.current < 250;
    if (
      shouldIgnoreScrollDepartureFromBottom(
        keepAutoScroll,
        stickToBottomRef.current,
        hadRecentUserScrollIntent,
      )
    ) {
      bubbleListRef.current?.scrollTo({ top: "bottom", behavior: "auto" });
      setShowScrollToBottom(false);
      return;
    }
    if (keepAutoScroll !== stickToBottomRef.current) {
      setStickToBottom(keepAutoScroll);
    }
    if (!hasOlderMessages || loading || loadingOlder) { return; }
    const distanceToHistoryTop = getDistanceToHistoryTop(
      target.scrollHeight,
      target.scrollTop,
      target.clientHeight,
      CHAT_SCROLL_IS_REVERSED,
    );
    if (distanceToHistoryTop > 24) { return; }
    void handleLoadOlderMessages();
  }, [handleLoadOlderMessages, hasOlderMessages, loading, loadingOlder]);

  const handleScrollToBottom = useCallback(() => {
    bubbleListRef.current?.scrollTo({ top: "bottom", behavior: "smooth" });
    setShowScrollToBottom(false);
    setStickToBottom(true);
  }, []);

  useEffect(() => {
    const scrollBox = scrollBoxRef.current;
    const scrollContent = scrollContentRef.current;
    if (!scrollBox || !scrollContent || typeof ResizeObserver === "undefined") { return; }

    scrollLayoutMetricsRef.current = {
      scrollHeight: scrollBox.scrollHeight,
      clientHeight: scrollBox.clientHeight,
    };

    let frameId = 0;

    const handleLayoutResize = () => {
      frameId = 0;
      const target = scrollBoxRef.current;
      if (!target) { return; }

      const nextMetrics = {
        scrollHeight: target.scrollHeight,
        clientHeight: target.clientHeight,
      };
      const previousMetrics = scrollLayoutMetricsRef.current;

      if (!hasScrollLayoutMetricsChanged(previousMetrics, nextMetrics)) {
        return;
      }

      scrollLayoutMetricsRef.current = nextMetrics;

      if (shouldStickToBottomOnLayoutChange(previousMetrics, nextMetrics, stickToBottomRef.current)) {
        bubbleListRef.current?.scrollTo({ top: "bottom", behavior: "auto" });
        setShowScrollToBottom(false);
        return;
      }

      syncScrollToBottomVisibility();
    };

    const observer = new ResizeObserver(() => {
      if (frameId) {
        window.cancelAnimationFrame(frameId);
      }
      frameId = window.requestAnimationFrame(handleLayoutResize);
    });

    observer.observe(scrollBox);
    observer.observe(scrollContent);

    return () => {
      observer.disconnect();
      if (frameId) {
        window.cancelAnimationFrame(frameId);
      }
    };
  }, [activeConversationId, bubbleListThemeKey, messageCount, syncScrollToBottomVisibility]);

  const prevStreamingRef = useRef(false);
  const streamingTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (streaming && !prevStreamingRef.current) {
      streamingTimerRef.current = setTimeout(() => {
        bubbleListRef.current?.scrollTo({ top: "bottom", behavior: "smooth" });
        setShowScrollToBottom(false);
        setStickToBottom(true);
      }, 50);
    }
    prevStreamingRef.current = streaming;
    return () => {
      if (streamingTimerRef.current) {
        clearTimeout(streamingTimerRef.current);
      }
    };
  }, [streaming]);

  useEffect(() => {
    const rafId = window.requestAnimationFrame(() => {
      if (stickToBottom) {
        bubbleListRef.current?.scrollTo({ top: "bottom", behavior: "auto" });
        setShowScrollToBottom(false);
        return;
      }
      syncScrollToBottomVisibility();
    });
    return () => window.cancelAnimationFrame(rafId);
  }, [allBubbleItems, stickToBottom, syncScrollToBottomVisibility]);

  useEffect(() => {
    if (!activeConversationId || allBubbleItems.length === 0) { return; }
    if (pendingScrollConversationIdRef.current !== activeConversationId) { return; }

    let frame1 = 0;
    let frame2 = 0;
    frame1 = window.requestAnimationFrame(() => {
      frame2 = window.requestAnimationFrame(() => {
        bubbleListRef.current?.scrollTo({ top: "bottom", behavior: "auto" });
        pendingScrollConversationIdRef.current = null;
      });
    });

    return () => {
      window.cancelAnimationFrame(frame1);
      window.cancelAnimationFrame(frame2);
    };
  }, [activeConversationId, allBubbleItems.length, lastBubbleKey]);

  return {
    showScrollToBottom,
    stickToBottom,
    scrollBoxRef,
    handleBubbleListScroll,
    handleScrollToBottom,
    minimapScrollTo,
    syncScrollToBottomVisibility,
    markUserScrollIntent,
    pendingScrollConversationIdRef,
  };
}
