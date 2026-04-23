import React, { useState, useEffect, useCallback } from 'react';
import { useMcpStore } from '@/stores';
import { FileText, Wrench, Terminal, Globe, BookOpen, FolderOpen } from 'lucide-react';

interface Suggestion {
  type: 'command' | 'tool' | 'file';
  label: string;
  description: string;
  replacement: string;
  icon: React.ReactNode;
}

// Slash commands
const SLASH_COMMANDS: Suggestion[] = [
  {
    type: 'command',
    label: '/search',
    description: 'Search the web',
    replacement: '/search ',
    icon: <Globe size={14} />,
  },
  {
    type: 'command',
    label: '/compact',
    description: 'Compress conversation context',
    replacement: '/compact',
    icon: <FileText size={14} />,
  },
  {
    type: 'command',
    label: '/clear',
    description: 'Clear conversation history',
    replacement: '/clear',
    icon: <Terminal size={14} />,
  },
  {
    type: 'command',
    label: '/tools',
    description: 'List available tools',
    replacement: '/tools',
    icon: <Wrench size={14} />,
  },
  {
    type: 'command',
    label: '/help',
    description: 'Show help and available commands',
    replacement: '/help',
    icon: <BookOpen size={14} />,
  },
];

interface CommandSuggestProps {
  value: string;
  cursorPosition: number;
  onSelect: (replacement: string) => void;
  visible: boolean;
}

const CommandSuggest: React.FC<CommandSuggestProps> = ({
  value,
  cursorPosition,
  onSelect,
  visible,
}) => {
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [triggerType, setTriggerType] = useState<'/' | '@' | null>(null);
  const mcpTools = useMcpStore((s) => s.toolDescriptors);

  // Parse the current input to detect trigger
  useEffect(() => {
    if (!visible) {
      setSuggestions([]);
      return;
    }

    const textBeforeCursor = value.slice(0, cursorPosition);
    // Find the last trigger character before cursor
    const lastSlash = textBeforeCursor.lastIndexOf('/');
    const lastAt = textBeforeCursor.lastIndexOf('@');

    // Determine which trigger is active (must be at start of line or after whitespace)
    let activeTrigger: '/' | '@' | null = null;
    let triggerPos = -1;
    let searchQuery = '';

    if (lastSlash >= 0) {
      const charBefore = lastSlash > 0 ? textBeforeCursor[lastSlash - 1] : '\n';
      if (charBefore === '\n' || charBefore === ' ' || charBefore === '') {
        const afterTrigger = textBeforeCursor.slice(lastSlash + 1);
        if (!afterTrigger.includes(' ') || lastSlash > lastAt) {
          activeTrigger = '/';
          triggerPos = lastSlash;
          searchQuery = afterTrigger.toLowerCase();
        }
      }
    }

    if (lastAt >= 0 && lastAt > triggerPos) {
      const charBefore = lastAt > 0 ? textBeforeCursor[lastAt - 1] : '\n';
      if (charBefore === '\n' || charBefore === ' ' || charBefore === '') {
        const afterTrigger = textBeforeCursor.slice(lastAt + 1);
        if (!afterTrigger.includes(' ')) {
          activeTrigger = '@';
          triggerPos = lastAt;
          searchQuery = afterTrigger.toLowerCase();
        }
      }
    }

    setTriggerType(activeTrigger);

    if (!activeTrigger) {
      setSuggestions([]);
      return;
    }

    if (activeTrigger === '/') {
      // Filter slash commands
      const filtered = SLASH_COMMANDS.filter(
        (cmd) => cmd.label.toLowerCase().includes(searchQuery) ||
                 cmd.description.toLowerCase().includes(searchQuery)
      );
      setSuggestions(filtered);
    } else if (activeTrigger === '@') {
      // @ mentions: tools and files
      const allTools = Object.values(mcpTools || {}).flat();
      const toolSuggestions: Suggestion[] = allTools
        .filter((tool: any) =>
          tool.name?.toLowerCase().includes(searchQuery) ||
          tool.description?.toLowerCase().includes(searchQuery)
        )
        .slice(0, 10)
        .map((tool: any) => ({
          type: 'tool' as const,
          label: tool.name,
          description: tool.description || 'MCP Tool',
          replacement: `@${tool.name} `,
          icon: <Wrench size={14} />,
        }));

      // Add file suggestion placeholder
      if (searchQuery.length > 0) {
        toolSuggestions.push({
          type: 'file',
          label: searchQuery,
          description: 'Reference file or path',
          replacement: `@${searchQuery} `,
          icon: <FolderOpen size={14} />,
        });
      }

      setSuggestions(toolSuggestions);
    }

    setSelectedIndex(0);
  }, [value, cursorPosition, visible, mcpTools]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (suggestions.length === 0) return false;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => (i + 1) % suggestions.length);
      return true;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => (i - 1 + suggestions.length) % suggestions.length);
      return true;
    }
    if (e.key === 'Enter' || e.key === 'Tab') {
      e.preventDefault();
      const selected = suggestions[selectedIndex];
      if (selected) {
        onSelect(selected.replacement);
      }
      return true;
    }
    if (e.key === 'Escape') {
      setSuggestions([]);
      return true;
    }
    return false;
  }, [suggestions, selectedIndex, onSelect]);

  // Expose key handler
  useEffect(() => {
    (CommandSuggest as any)._handleKeyDown = handleKeyDown;
  }, [handleKeyDown]);

  if (suggestions.length === 0 || !visible) return null;

  return (
    <div className="absolute bottom-full left-0 right-0 mb-1 max-h-48 overflow-y-auto bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg z-50">
      <div className="px-2 py-1 text-xs text-gray-500 dark:text-gray-400 border-b border-gray-100 dark:border-gray-700">
        {triggerType === '/' ? 'Commands' : 'Mentions'} — type to filter, ↑↓ to select, Enter to confirm
      </div>
      {suggestions.map((suggestion, index) => (
        <button
          key={`${suggestion.type}-${suggestion.label}`}
          className={`w-full flex items-center gap-2 px-3 py-1.5 text-sm text-left transition-colors ${
            index === selectedIndex
              ? 'bg-blue-50 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300'
              : 'text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700/50'
          }`}
          onClick={() => onSelect(suggestion.replacement)}
          onMouseEnter={() => setSelectedIndex(index)}
        >
          <span className="flex-shrink-0 text-gray-400">{suggestion.icon}</span>
          <span className="font-medium">{suggestion.label}</span>
          <span className="text-xs text-gray-400 dark:text-gray-500 truncate">{suggestion.description}</span>
        </button>
      ))}
    </div>
  );
};

// Export a hook to check if the suggest is handling a key event
export const isCommandSuggestHandlingKey = (): boolean => {
  return false; // Will be managed by parent
};

export default CommandSuggest;
