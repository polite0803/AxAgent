import { Terminal as XTerm } from "@xterm/xterm";
import { useCallback, useEffect, useRef, useState } from "react";

export interface CommandSuggestion {
  command: string;
  description: string;
  parameters?: CommandParameter[];
}

export interface CommandParameter {
  name: string;
  description: string;
  required: boolean;
  default?: string;
}

export interface SlashCompleterOptions {
  commands: CommandSuggestion[];
  triggerChar?: string;
  maxSuggestions?: number;
  onSelect?: (command: string, params?: Record<string, string>) => void;
}

export function useSlashCompleter(
  terminal: XTerm | null,
  options: SlashCompleterOptions,
) {
  const {
    commands,
    triggerChar = "/",
    maxSuggestions = 10,
    onSelect,
  } = options;

  const [isActive, setIsActive] = useState(false);
  const [currentInput, setCurrentInput] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [_cursorPosition, _setCursorPosition] = useState(0);
  const inputBufferRef = useRef("");
  const cursorPosRef = useRef(0);

  const getFilteredCommands = useCallback(
    (input: string) => {
      if (!input.startsWith(triggerChar)) { return []; }
      const searchTerm = input.slice(1).toLowerCase();
      return commands
        .filter(
          (cmd) =>
            cmd.command.toLowerCase().startsWith(searchTerm)
            || cmd.command.toLowerCase().includes(searchTerm),
        )
        .slice(0, maxSuggestions);
    },
    [commands, triggerChar, maxSuggestions],
  );

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (!terminal) { return; }

      const filteredCommands = getFilteredCommands(inputBufferRef.current);

      if (event.key === triggerChar && !isActive) {
        inputBufferRef.current = triggerChar;
        cursorPosRef.current = terminal.buffer.active.cursorX;
        setIsActive(true);
        setCurrentInput(triggerChar);
        setSelectedIndex(0);
        return;
      }

      if (isActive) {
        if (event.key === "Escape") {
          inputBufferRef.current = "";
          setIsActive(false);
          setCurrentInput("");
          event.preventDefault();
          return;
        }

        if (event.key === "ArrowDown") {
          setSelectedIndex((prev) => Math.min(prev + 1, filteredCommands.length - 1));
          event.preventDefault();
          return;
        }

        if (event.key === "ArrowUp") {
          setSelectedIndex((prev) => Math.max(prev - 1, 0));
          event.preventDefault();
          return;
        }

        if (event.key === "Tab" || event.key === "Enter") {
          if (filteredCommands.length > 0) {
            const selected = filteredCommands[selectedIndex];
            if (selected) {
              insertCommand(selected.command);
              if (onSelect) {
                onSelect(selected.command);
              }
            }
          } else if (event.key === "Enter" && currentInput.length > 1) {
            setIsActive(false);
          }
          event.preventDefault();
          return;
        }

        if (event.key === "Backspace") {
          if (inputBufferRef.current.length > 1) {
            inputBufferRef.current = inputBufferRef.current.slice(0, -1);
            setCurrentInput(inputBufferRef.current);
            setSelectedIndex(0);
          } else {
            setIsActive(false);
            inputBufferRef.current = "";
          }
          return;
        }

        if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) {
          inputBufferRef.current += event.key;
          setCurrentInput(inputBufferRef.current);
          setSelectedIndex(0);
        }
      }
    },
    [terminal, isActive, currentInput, selectedIndex, getFilteredCommands, triggerChar, onSelect],
  );

  const insertCommand = useCallback(
    (command: string) => {
      if (!terminal) { return; }
      for (let i = 0; i < inputBufferRef.current.length; i++) {
        terminal.write("\b \b");
      }
      terminal.write(command + " ");
      inputBufferRef.current = "";
      setIsActive(false);
      setCurrentInput("");
    },
    [terminal],
  );

  const clearInput = useCallback(() => {
    if (!terminal) { return; }
    for (let i = 0; i < inputBufferRef.current.length; i++) {
      terminal.write("\b \b");
    }
    inputBufferRef.current = "";
    setIsActive(false);
    setCurrentInput("");
  }, [terminal]);

  useEffect(() => {
    if (terminal) {
      const handler = (event: KeyboardEvent) => {
        handleKeyDown(event);
      };
      terminal.element?.addEventListener("keydown", handler);
      return () => {
        terminal.element?.removeEventListener("keydown", handler);
      };
    }
  }, [terminal, handleKeyDown]);

  return {
    isActive,
    currentInput,
    selectedIndex,
    filteredCommands: getFilteredCommands(currentInput),
    clearInput,
    insertCommand,
  };
}

export interface SlashCompleterWidgetProps {
  terminal: XTerm | null;
  commands: CommandSuggestion[];
  triggerChar?: string;
  onSelect?: (command: string) => void;
  style?: React.CSSProperties;
}

export function SlashCompleterWidget({
  terminal,
  commands,
  triggerChar = "/",
  onSelect,
  style,
}: SlashCompleterWidgetProps) {
  const {
    isActive,
    currentInput: _currentInput,
    filteredCommands,
    selectedIndex,
  } = useSlashCompleter(
    terminal,
    {
      commands,
      triggerChar,
      onSelect,
    },
  );

  if (!isActive || filteredCommands.length === 0) {
    return null;
  }

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
        minWidth: 250,
        ...style,
      }}
    >
      {filteredCommands.map((cmd, index) => (
        <div
          key={cmd.command}
          style={{
            padding: "6px 12px",
            cursor: "pointer",
            background: index === selectedIndex ? "#585b70" : "transparent",
            borderLeft: index === selectedIndex ? "2px solid #89b4fa" : "2px solid transparent",
          }}
          onMouseEnter={() => {}}
        >
          <div style={{ color: "#cdd6f4", fontSize: 13 }}>
            <span style={{ color: "#89b4fa" }}>{triggerChar}</span>
            {cmd.command.slice(1)}
          </div>
          {cmd.description && (
            <div
              style={{
                color: "#6c7086",
                fontSize: 11,
                marginTop: 2,
              }}
            >
              {cmd.description}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

export const DEFAULT_COMMANDS: CommandSuggestion[] = [
  {
    command: "/help",
    description: "Show available commands",
  },
  {
    command: "/clear",
    description: "Clear the terminal screen",
  },
  {
    command: "/reset",
    description: "Reset the terminal session",
  },
  {
    command: "/status",
    description: "Show current session status",
  },
  {
    command: "/history",
    description: "Show command history",
  },
  {
    command: "/env",
    description: "Show environment variables",
  },
  {
    command: "/theme",
    description: "Change terminal theme",
    parameters: [
      { name: "theme-name", description: "Theme name", required: true },
    ],
  },
  {
    command: "/export",
    description: "Export session data",
    parameters: [
      { name: "format", description: "Export format (json/csv)", required: false, default: "json" },
    ],
  },
];
