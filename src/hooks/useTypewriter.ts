// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useEffect, useRef, useState } from "react";

export interface UseTypewriterOptions {
  text: string;
  speed?: number;
  delay?: number;
  onComplete?: () => void;
  autoStart?: boolean;
}

export interface UseTypewriterReturn {
  displayText: string;
  isTyping: boolean;
  isComplete: boolean;
  start: () => void;
  pause: () => void;
  resume: () => void;
  reset: () => void;
  skip: () => void;
  progress: number;
}

export function useTypewriter(options: UseTypewriterOptions): UseTypewriterReturn {
  const {
    text,
    speed = 16,
    delay = 0,
    onComplete,
    autoStart = true,
  } = options;

  const [displayText, setDisplayText] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const [isComplete, setIsComplete] = useState(false);
  const [progress, setProgress] = useState(0);
  const indexRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pausedRef = useRef(false);

  const clearTimer = useCallback(() => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  const tick = useCallback(() => {
    if (pausedRef.current) { return; }
    if (indexRef.current >= text.length) {
      setIsTyping(false);
      setIsComplete(true);
      setProgress(1);
      onComplete?.();
      return;
    }

    const nextChar = text[indexRef.current];
    indexRef.current += 1;

    setDisplayText(text.slice(0, indexRef.current));
    setProgress(indexRef.current / Math.max(text.length, 1));

    if (nextChar === "\n") {
      timerRef.current = setTimeout(tick, speed * 5);
    } else if (nextChar === "." || nextChar === "!" || nextChar === "?") {
      timerRef.current = setTimeout(tick, speed * 8);
    } else if (nextChar === "," || nextChar === ";") {
      timerRef.current = setTimeout(tick, speed * 3);
    } else {
      timerRef.current = setTimeout(tick, speed);
    }
  }, [text, speed, onComplete]);

  const start = useCallback(() => {
    clearTimer();
    setIsComplete(false);
    setIsTyping(true);
    pausedRef.current = false;
    indexRef.current = 0;
    setDisplayText("");
    setProgress(0);

    timerRef.current = setTimeout(() => {
      tick();
    }, delay);
  }, [delay, tick, clearTimer]);

  const pause = useCallback(() => {
    pausedRef.current = true;
    setIsTyping(false);
    clearTimer();
  }, [clearTimer]);

  const resume = useCallback(() => {
    if (isComplete) { return; }
    pausedRef.current = false;
    setIsTyping(true);
    tick();
  }, [isComplete, tick]);

  const reset = useCallback(() => {
    clearTimer();
    pausedRef.current = false;
    indexRef.current = 0;
    setDisplayText("");
    setIsTyping(false);
    setIsComplete(false);
    setProgress(0);
  }, [clearTimer]);

  const skip = useCallback(() => {
    clearTimer();
    pausedRef.current = false;
    setDisplayText(text);
    setProgress(1);
    setIsTyping(false);
    setIsComplete(true);
    indexRef.current = text.length;
    onComplete?.();
  }, [text, onComplete, clearTimer]);

  useEffect(() => {
    if (!autoStart) { return; }
    if (text.length === 0) {
      reset();
    } else if (!isComplete && !isTyping) {
      // 流式追加场景：如果已有部分文本，从断点继续打字
      if (displayText.length < text.length) {
        // 直接追加显示剩余文本（流式场景不做逐字动画以避免延迟）
        setDisplayText(text);
        setProgress(1);
        setIsComplete(true);
        setIsTyping(false);
        onComplete?.();
      }
    }
  }, [text, autoStart, isComplete, isTyping, displayText, reset, onComplete]);

  useEffect(() => {
    return () => {
      clearTimer();
    };
  }, [clearTimer]);

  return {
    displayText,
    isTyping,
    isComplete,
    start,
    pause,
    resume,
    reset,
    skip,
    progress,
  };
}
