import React, { createContext, useCallback, useContext, useMemo, useRef } from "react";

type ScrollToFn = (messageId: string) => void;

interface ScrollToMessageContextValue {
  scrollTo: ScrollToFn;
  scrollBoxRef: React.RefObject<HTMLElement | null>;
  /** Set by programmatic scroll — suppresses detection updates */
  scrollLockRef: React.MutableRefObject<number>;
  /** Forced active ID set by click — overrides detection during lock */
  forcedActiveRef: React.MutableRefObject<string | null>;
}

const Context = createContext<ScrollToMessageContextValue | null>(null);

export const MinimapScrollProvider = ScrollToMessageProvider;
export function ScrollToMessageProvider({
  children,
  scrollTo,
  scrollBoxRef,
}: {
  children: React.ReactNode;
  scrollTo: ScrollToFn;
  scrollBoxRef: React.RefObject<HTMLElement | null>;
}) {
  const scrollLockRef = useRef(0);
  const forcedActiveRef = useRef<string | null>(null);
  const wrappedScrollTo = useCallback<ScrollToFn>(
    (messageId) => {
      scrollLockRef.current = Date.now() + 800;
      forcedActiveRef.current = messageId;
      scrollTo(messageId);
    },
    [scrollTo],
  );
  const value = useMemo(
    () => ({ scrollTo: wrappedScrollTo, scrollBoxRef, scrollLockRef, forcedActiveRef }),
    [wrappedScrollTo, scrollBoxRef],
  );
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

export function useScrollToMessage(): ScrollToMessageContextValue {
  const ctx = useContext(Context);
  return (
    ctx ?? {
      scrollTo: () => {},
      scrollBoxRef: { current: null },
      scrollLockRef: { current: 0 },
      forcedActiveRef: { current: null },
    }
  );
}
