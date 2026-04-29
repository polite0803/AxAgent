import { useCallback, useEffect, useRef, useState } from "react";
import { Terminal as XTerm } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { SearchAddon } from "@xterm/addon-search";
import { Unicode11Addon } from "@xterm/addon-unicode11";

export interface ITerminalTheme {
  background: string;
  foreground: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

export interface XtermEnhancementOptions {
  cursorBlink?: boolean;
  fontSize?: number;
  fontFamily?: string;
  theme?: ITerminalTheme;
  scrollback?: number;
  enableWebLinks?: boolean;
  enableSearch?: boolean;
  onReady?: (terminal: XTerm, fitAddon: FitAddon) => void;
}

export function useXtermEnhancement(
  containerRef: React.RefObject<HTMLDivElement | null>,
  options: XtermEnhancementOptions = {}
) {
  const {
    cursorBlink = true,
    fontSize = 14,
    fontFamily = "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
    theme,
    scrollback = 10000,
    enableWebLinks = true,
    enableSearch = true,
    onReady,
  } = options;

  const terminalRef = useRef<XTerm | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const searchAddonRef = useRef<SearchAddon | null>(null);
  const [isReady, setIsReady] = useState(false);

  const defaultTheme: ITerminalTheme = {
    background: "#1e1e2e",
    foreground: "#cdd6f4",
    cursor: "#f5e0dc",
    cursorAccent: "#1e1e2e",
    selectionBackground: "#585b7066",
    black: "#45475a",
    red: "#f38ba8",
    green: "#a6e3a1",
    yellow: "#f9e2af",
    blue: "#89b4fa",
    magenta: "#f5c2e7",
    cyan: "#94e2d5",
    white: "#bac2de",
    brightBlack: "#585b70",
    brightRed: "#f38ba8",
    brightGreen: "#a6e3a1",
    brightYellow: "#f9e2af",
    brightBlue: "#89b4fa",
    brightMagenta: "#f5c2e7",
    brightCyan: "#94e2d5",
    brightWhite: "#a6adc8",
  };

  const initTerminal = useCallback(async () => {
    if (!containerRef.current || terminalRef.current) return;

    const xterm = new XTerm({
      cursorBlink,
      fontSize,
      fontFamily,
      theme: theme || defaultTheme,
      scrollback,
      convertEol: true,
    });

    const fitAddon = new FitAddon();
    xterm.loadAddon(fitAddon);

    if (enableWebLinks) {
      xterm.loadAddon(new WebLinksAddon());
    }

    if (enableSearch) {
      const searchAddon = new SearchAddon();
      xterm.loadAddon(searchAddon);
      searchAddonRef.current = searchAddon;
    }

    xterm.loadAddon(new Unicode11Addon());

    xterm.open(containerRef.current);
    fitAddon.fit();

    terminalRef.current = xterm;
    fitAddonRef.current = fitAddon;
    setIsReady(true);

    if (onReady) {
      onReady(xterm, fitAddon);
    }

    return () => {
      xterm.dispose();
      terminalRef.current = null;
      fitAddonRef.current = null;
      searchAddonRef.current = null;
      setIsReady(false);
    };
  }, [containerRef, cursorBlink, fontSize, fontFamily, theme, scrollback, enableWebLinks, enableSearch, onReady]);

  const fit = useCallback(() => {
    if (fitAddonRef.current) {
      fitAddonRef.current.fit();
    }
  }, []);

  const search = useCallback((term: string, options?: { regex?: boolean; wholeWord?: boolean; caseSensitive?: boolean }) => {
    if (searchAddonRef.current) {
      searchAddonRef.current.findNext(term, options);
    }
  }, []);

  const searchPrevious = useCallback((term: string, options?: { regex?: boolean; wholeWord?: boolean; caseSensitive?: boolean }) => {
    if (searchAddonRef.current) {
      searchAddonRef.current.findPrevious(term, options);
    }
  }, []);

  useEffect(() => {
    let cleanupFn: (() => void) | undefined;
    initTerminal().then((cleanup) => {
      cleanupFn = cleanup;
    });
    return () => {
      if (cleanupFn) {
        cleanupFn();
      }
    };
  }, [initTerminal]);

  return {
    terminal: terminalRef.current,
    fitAddon: fitAddonRef.current,
    isReady,
    fit,
    search,
    searchPrevious,
  };
}

export function useOscClipboard(terminal: XTerm | null) {
  const handleClipboard = useCallback(async (event: ClipboardEvent) => {
    if (event.type === "copy" && terminal) {
      const selection = terminal.getSelection();
      if (selection) {
        event.clipboardData?.setData("text/plain", selection);
        event.preventDefault();
      }
    }
  }, [terminal]);

  const copySelection = useCallback(() => {
    if (terminal) {
      const selection = terminal.getSelection();
      if (selection) {
        navigator.clipboard.writeText(selection);
      }
    }
  }, [terminal]);

  const paste = useCallback(async () => {
    if (terminal) {
      try {
        const text = await navigator.clipboard.readText();
        terminal.paste(text);
      } catch (e) {
        console.error("Failed to paste from clipboard:", e);
      }
    }
  }, [terminal]);

  useEffect(() => {
    document.addEventListener("copy", handleClipboard);
    document.addEventListener("cut", handleClipboard);

    return () => {
      document.removeEventListener("copy", handleClipboard);
      document.removeEventListener("cut", handleClipboard);
    };
  }, [handleClipboard]);

  return { copySelection, paste };
}

export function useVirtualScroll(
  terminal: XTerm | null,
  _bufferSize: number = 5000
) {
  const [visibleRange, _setVisibleRange] = useState({ start: 0, end: 0 });

  const scrollToBottom = useCallback(() => {
    if (terminal) {
      terminal.scrollToBottom();
    }
  }, [terminal]);

  const scrollToTop = useCallback(() => {
    if (terminal) {
      terminal.scrollToTop();
    }
  }, [terminal]);

  const scrollLines = useCallback((lines: number) => {
    if (terminal) {
      terminal.scrollLines(lines);
    }
  }, [terminal]);

  const onScroll = useCallback((callback: (scrollTop: number, scrollBottom: number) => void) => {
    if (terminal) {
      terminal.onScroll((newScrollTop) => {
        callback(newScrollTop, newScrollTop + terminal.rows);
      });
    }
  }, [terminal]);

  return {
    visibleRange,
    scrollToBottom,
    scrollToTop,
    scrollLines,
    onScroll,
  };
}

export interface EnhancementCompositorProps {
  children: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
}

export function XtermEnhancementCompositor({
  children,
  className,
  style,
}: EnhancementCompositorProps) {
  return (
    <div className={className} style={style}>
      {children}
    </div>
  );
}
