import { invoke } from "@/lib/invoke";
import { Terminal as XTerm } from "@xterm/xterm";
import { useCallback, useEffect, useRef, useState } from "react";

export interface PathCompleterOptions {
  triggerKey?: string;
  maxSuggestions?: number;
  onPathSelected?: (path: string) => void;
  getSuggestions?: (input: string) => Promise<string[]>;
}

export function usePathCompleter(
  terminal: XTerm | null,
  options: PathCompleterOptions = {},
) {
  const {
    triggerKey = "Tab",
    maxSuggestions = 20,
    onPathSelected,
    getSuggestions,
  } = options;

  const [isActive, setIsActive] = useState(false);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [currentInput, setCurrentInput] = useState("");
  const inputBufferRef = useRef("");
  const completionTriggeredRef = useRef(false);

  const defaultGetSuggestions = useCallback(
    async (input: string): Promise<string[]> => {
      if (!input || input.length < 1) { return []; }

      const lastSpaceIndex = input.lastIndexOf(" ");
      const searchBase = lastSpaceIndex >= 0 ? input.slice(lastSpaceIndex + 1) : input;

      if (!searchBase.includes("/") && !searchBase.includes("\\") && !searchBase.includes(".")) {
        return [];
      }

      try {
        const result = await invoke<string[]>(
          "path_complete",
          { partialPath: searchBase },
        );
        return result || [];
      } catch {
        return [];
      }
    },
    [],
  );

  const fetchSuggestions = useCallback(
    async (input: string) => {
      const fetcher = getSuggestions || defaultGetSuggestions;
      const results = await fetcher(input);
      setSuggestions(results.slice(0, maxSuggestions));
      setSelectedIndex(0);
    },
    [getSuggestions, defaultGetSuggestions, maxSuggestions],
  );

  const insertPath = useCallback(
    (path: string) => {
      if (!terminal) { return; }

      const lastSpaceIndex = inputBufferRef.current.lastIndexOf(" ");
      const beforeSpace = lastSpaceIndex >= 0 ? inputBufferRef.current.slice(0, lastSpaceIndex + 1) : "";

      for (let i = 0; i < inputBufferRef.current.length - beforeSpace.length; i++) {
        terminal.write("\b \b");
      }

      terminal.write(path + " ");
      inputBufferRef.current = beforeSpace + path + " ";
      setIsActive(false);
      setSuggestions([]);

      if (onPathSelected) {
        onPathSelected(path);
      }
    },
    [terminal, onPathSelected],
  );

  const cycleSuggestion = useCallback(
    (direction: 1 | -1) => {
      if (suggestions.length === 0) { return; }
      setSelectedIndex(
        (prev) => (prev + direction + suggestions.length) % suggestions.length,
      );
    },
    [suggestions.length],
  );

  const acceptSuggestion = useCallback(() => {
    if (suggestions.length > 0 && selectedIndex < suggestions.length) {
      insertPath(suggestions[selectedIndex]);
    }
  }, [suggestions, selectedIndex, insertPath]);

  const cancelCompletion = useCallback(() => {
    setIsActive(false);
    setSuggestions([]);
    completionTriggeredRef.current = false;
  }, []);

  useEffect(() => {
    if (!terminal) { return; }

    const handleData = (data: string) => {
      if (isActive) {
        inputBufferRef.current += data;
        setCurrentInput(inputBufferRef.current);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === triggerKey && !completionTriggeredRef.current) {
        const selection = terminal.getSelection();
        if (selection && !selection.includes(" ")) {
          inputBufferRef.current = selection;
          setCurrentInput(selection);
          completionTriggeredRef.current = true;
          setIsActive(true);
          fetchSuggestions(selection);
          event.preventDefault();
          return;
        }

        if (inputBufferRef.current.length > 0) {
          completionTriggeredRef.current = true;
          setIsActive(true);
          fetchSuggestions(inputBufferRef.current);
          event.preventDefault();
        }
      }

      if (isActive) {
        if (event.key === "Escape") {
          cancelCompletion();
          event.preventDefault();
          return;
        }

        if (event.key === "ArrowDown") {
          cycleSuggestion(1);
          event.preventDefault();
          return;
        }

        if (event.key === "ArrowUp") {
          cycleSuggestion(-1);
          event.preventDefault();
          return;
        }

        if (event.key === triggerKey) {
          acceptSuggestion();
          event.preventDefault();
          return;
        }

        if (event.key === "Enter") {
          if (suggestions.length > 0) {
            acceptSuggestion();
          } else {
            cancelCompletion();
          }
          event.preventDefault();
          return;
        }

        if (event.key === "Backspace") {
          if (inputBufferRef.current.length > 0) {
            inputBufferRef.current = inputBufferRef.current.slice(0, -1);
            setCurrentInput(inputBufferRef.current);
            if (inputBufferRef.current.length > 0) {
              fetchSuggestions(inputBufferRef.current);
            } else {
              cancelCompletion();
            }
          } else {
            cancelCompletion();
          }
          event.preventDefault();
          return;
        }

        if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) {
          inputBufferRef.current += event.key;
          setCurrentInput(inputBufferRef.current);
          fetchSuggestions(inputBufferRef.current);
        }
      }
    };

    const disposeHandler = terminal.onData(handleData);
    terminal.element?.addEventListener("keydown", handleKeyDown);

    return () => {
      disposeHandler.dispose();
      terminal.element?.removeEventListener("keydown", handleKeyDown);
    };
  }, [
    terminal,
    isActive,
    triggerKey,
    fetchSuggestions,
    cycleSuggestion,
    acceptSuggestion,
    cancelCompletion,
    suggestions.length,
  ]);

  return {
    isActive,
    currentInput,
    suggestions,
    selectedIndex,
    insertPath,
    acceptSuggestion,
    cancelCompletion,
  };
}

export interface PathCompleterWidgetProps {
  terminal: XTerm | null;
  style?: React.CSSProperties;
}

export function PathCompleterWidget({
  terminal,
  style,
}: PathCompleterWidgetProps) {
  const {
    isActive,
    suggestions,
    selectedIndex,
  } = usePathCompleter(terminal, {});

  if (!isActive || suggestions.length === 0) {
    return null;
  }

  const displaySuggestions = suggestions.slice(0, 10);

  return (
    <div
      style={{
        position: "absolute",
        bottom: 24,
        left: 8,
        background: "#181825",
        border: "1px solid #45475a",
        borderRadius: 6,
        maxHeight: 200,
        overflow: "auto",
        zIndex: 1000,
        minWidth: 300,
        ...style,
      }}
    >
      {displaySuggestions.map((path, index) => (
        <div
          key={path}
          style={{
            padding: "4px 12px",
            cursor: "pointer",
            background: index === selectedIndex ? "#585b70" : "transparent",
            fontSize: 12,
            fontFamily: "'JetBrains Mono', monospace",
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          <span style={{ color: "#89b4fa" }}>{"📄 "}</span>
          <span style={{ color: "#cdd6f4" }}>{path}</span>
        </div>
      ))}
      {suggestions.length > 10 && (
        <div
          style={{
            padding: "4px 12px",
            color: "#6c7086",
            fontSize: 11,
            borderTop: "1px solid #45475a",
          }}
        >
          ...and {suggestions.length - 10} more (scroll for more)
        </div>
      )}
    </div>
  );
}

export function useSmartPathCompletion(_terminal: XTerm | null) {
  const [isCompleting, setIsCompleting] = useState(false);
  const [completions, setCompletions] = useState<string[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);

  const startCompletion = useCallback(
    async (word: string) => {
      setIsCompleting(true);
      try {
        const results = await invoke<string[]>(
          "path_complete",
          { partialPath: word },
        );
        setCompletions(results || []);
        setCurrentIndex(0);
      } catch {
        setCompletions([]);
      }
    },
    [],
  );

  const nextCompletion = useCallback(() => {
    setCurrentIndex((prev) => (prev + 1) % Math.max(completions.length, 1));
  }, [completions.length]);

  const prevCompletion = useCallback(() => {
    setCurrentIndex(
      (prev) => (prev - 1 + completions.length) % Math.max(completions.length, 1),
    );
  }, [completions.length]);

  const insertCurrentCompletion = useCallback(
    (terminal: XTerm | null) => {
      if (!terminal || completions.length === 0) { return; }
      terminal.write(completions[currentIndex]);
      setIsCompleting(false);
      setCompletions([]);
    },
    [completions, currentIndex],
  );

  const cancel = useCallback(() => {
    setIsCompleting(false);
    setCompletions([]);
    setCurrentIndex(0);
  }, []);

  return {
    isCompleting,
    completions,
    currentIndex,
    startCompletion,
    nextCompletion,
    prevCompletion,
    insertCurrentCompletion,
    cancel,
  };
}
