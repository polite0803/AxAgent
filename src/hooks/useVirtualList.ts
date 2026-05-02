import { useVirtualizer, type VirtualItem } from "@tanstack/react-virtual";
import { useCallback, useEffect, useRef, useState } from "react";

interface UseVirtualListOptions {
  itemCount: number;
  estimateSize: (index: number) => number;
  overscan?: number;
  gap?: number;
}

interface UseVirtualListReturn {
  parentRef: React.RefObject<HTMLDivElement>;
  virtualItems: VirtualItem[];
  totalSize: number;
  scrollToIndex: (index: number) => void;
  scrollToTop: () => void;
  isAtBottom: boolean;
}

export function useVirtualList({
  itemCount,
  estimateSize,
  overscan = 5,
  gap = 0,
}: UseVirtualListOptions): UseVirtualListReturn {
  const parentRef = useRef<HTMLDivElement>(null!);
  const [isAtBottom, setIsAtBottom] = useState(false);

  const virtualizer = useVirtualizer({
    count: itemCount,
    getScrollElement: () => parentRef.current,
    estimateSize,
    overscan,
    gap,
  });

  const virtualItems = virtualizer.getVirtualItems();
  const totalSize = virtualizer.getTotalSize();

  const scrollToIndex = useCallback(
    (index: number) => {
      virtualizer.scrollToIndex(index, { align: "start" });
    },
    [virtualizer],
  );

  const scrollToTop = useCallback(() => {
    virtualizer.scrollToIndex(0, { align: "start" });
  }, [virtualizer]);

  useEffect(() => {
    const el = parentRef.current;
    if (!el) { return; }

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = el;
      setIsAtBottom(scrollHeight - scrollTop - clientHeight < 50);
    };

    el.addEventListener("scroll", handleScroll, { passive: true });
    return () => el.removeEventListener("scroll", handleScroll);
  }, []);

  return {
    parentRef,
    virtualItems,
    totalSize,
    scrollToIndex,
    scrollToTop,
    isAtBottom,
  };
}

export function useChatVirtualList(messageCount: number) {
  return useVirtualList({
    itemCount: messageCount,
    estimateSize: () => 120,
    overscan: 3,
    gap: 8,
  });
}

export function useConversationVirtualList(conversationCount: number) {
  return useVirtualList({
    itemCount: conversationCount,
    estimateSize: () => 56,
    overscan: 10,
    gap: 2,
  });
}
