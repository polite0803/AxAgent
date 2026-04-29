import {
  Button,
  Card,
  Col,
  Input,
  List,
  Modal,
  message,
  Row,
  Space,
  Tag,
  Typography,
} from "antd";
import { Keyboard, Save } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

export interface KeyboardShortcut {
  id: string;
  category: string;
  name: string;
  defaultKey: string;
  currentKey?: string;
  description?: string;
}

interface KeyboardShortcutsManagerProps {
  visible: boolean;
  onClose: () => void;
}

const DEFAULT_SHORTCUTS: KeyboardShortcut[] = [
  { id: "new-session", category: "Session", name: "New Session", defaultKey: "Ctrl+Shift+N", description: "Create a new chat session" },
  { id: "search", category: "General", name: "Search", defaultKey: "Ctrl+K", description: "Open search panel" },
  { id: "settings", category: "General", name: "Settings", defaultKey: "Ctrl+,", description: "Open settings" },
  { id: "toggle-theme", category: "Appearance", name: "Toggle Theme", defaultKey: "Ctrl+Shift+T", description: "Switch between light/dark theme" },
  { id: "command-palette", category: "General", name: "Command Palette", defaultKey: "Ctrl+Shift+P", description: "Open command palette" },
  { id: "terminal", category: "Terminal", name: "Toggle Terminal", defaultKey: "Ctrl+`", description: "Show/hide terminal panel" },
  { id: "clear-chat", category: "Chat", name: "Clear Chat", defaultKey: "Ctrl+Shift+C", description: "Clear current chat history" },
  { id: "export-chat", category: "Chat", name: "Export Chat", defaultKey: "Ctrl+Shift+E", description: "Export chat as markdown" },
  { id: "focus-input", category: "Chat", name: "Focus Input", defaultKey: "Escape", description: "Focus chat input field" },
  { id: "toggle-sidebar", category: "Layout", name: "Toggle Sidebar", defaultKey: "Ctrl+B", description: "Show/hide sidebar" },
  { id: "quick-command", category: "Chat", name: "Quick Command", defaultKey: "/", description: "Open slash command menu" },
  { id: "interrupt", category: "Agent", name: "Interrupt Agent", defaultKey: "Ctrl+C", description: "Interrupt running agent" },
];

const SHORTCUT_CATEGORIES = ["General", "Session", "Chat", "Terminal", "Appearance", "Layout", "Agent"];

export default function KeyboardShortcutsManager({
  visible,
  onClose,
}: KeyboardShortcutsManagerProps) {
  const { t } = useTranslation();
  const [shortcuts, setShortcuts] = useState<KeyboardShortcut[]>([]);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editKey, setEditKey] = useState("");

  useEffect(() => {
    const stored = localStorage.getItem("axagent-keyboard-shortcuts");
    if (stored) {
      try {
        const parsed = JSON.parse(stored);
        const merged = DEFAULT_SHORTCUTS.map((def) => {
          const saved = parsed.find((s: KeyboardShortcut) => s.id === def.id);
          return saved ? { ...def, currentKey: saved.currentKey || def.defaultKey } : def;
        });
        setShortcuts(merged);
      } catch {
        setShortcuts(DEFAULT_SHORTCUTS.map(s => ({ ...s, currentKey: s.defaultKey })));
      }
    } else {
      setShortcuts(DEFAULT_SHORTCUTS.map(s => ({ ...s, currentKey: s.defaultKey })));
    }
  }, [visible]);

  const handleStartEdit = (shortcut: KeyboardShortcut) => {
    setEditingId(shortcut.id);
    setEditKey(shortcut.currentKey || shortcut.defaultKey);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!editingId) return;

    e.preventDefault();
    e.stopPropagation();

    const keys: string[] = [];
    if (e.ctrlKey) keys.push("Ctrl");
    if (e.altKey) keys.push("Alt");
    if (e.shiftKey) keys.push("Shift");
    if (e.metaKey) keys.push("Meta");

    const key = e.key;
    if (!["Control", "Alt", "Shift", "Meta"].includes(key)) {
      keys.push(key.length === 1 ? key.toUpperCase() : key);
    }

    setEditKey(keys.join("+"));
  };

  const handleSaveEdit = () => {
    if (!editingId) return;

    const updated = shortcuts.map((s) =>
      s.id === editingId ? { ...s, currentKey: editKey } : s
    );
    setShortcuts(updated);
    setEditingId(null);
    setEditKey("");

    const toSave = updated.map(({ id, currentKey }) => ({ id, currentKey }));
    localStorage.setItem("axagent-keyboard-shortcuts", JSON.stringify(toSave));
    message.success(t("settings.shortcuts.saved"));
  };

  const handleCancelEdit = () => {
    setEditingId(null);
    setEditKey("");
  };

  const handleResetAll = () => {
    setShortcuts(DEFAULT_SHORTCUTS.map((s) => ({ ...s, currentKey: s.defaultKey })));
    localStorage.removeItem("axagent-keyboard-shortcuts");
    message.success(t("settings.shortcuts.reset"));
  };

  const groupedShortcuts = SHORTCUT_CATEGORIES.reduce((acc, category) => {
    const items = shortcuts.filter((s) => s.category === category);
    if (items.length > 0) {
      acc[category] = items;
    }
    return acc;
  }, {} as Record<string, KeyboardShortcut[]>);

  return (
    <Modal
      title={
        <Space>
          <Keyboard size={18} />
          <span>{t("settings.shortcuts.title")}</span>
        </Space>
      }
      open={visible}
      onCancel={onClose}
      width={700}
      footer={null}
    >
      <div style={{ padding: "8px 0" }}>
        <div style={{ marginBottom: 16 }}>
          <Paragraph type="secondary">
            {t("settings.shortcuts.description")}
          </Paragraph>
        </div>

        <Space style={{ marginBottom: 16 }}>
          <Button onClick={handleResetAll} danger>
            {t("settings.shortcuts.resetAll")}
          </Button>
        </Space>

        <Row gutter={[16, 16]}>
          {Object.entries(groupedShortcuts).map(([category, items]) => (
            <Col span={24} key={category}>
              <Card size="small" title={category}>
                <List
                  size="small"
                  dataSource={items}
                  renderItem={(shortcut) => (
                    <List.Item
                      style={{
                        padding: "8px 0",
                        cursor: "pointer",
                      }}
                      onClick={() => handleStartEdit(shortcut)}
                    >
                      <div
                        style={{
                          display: "flex",
                          justifyContent: "space-between",
                          alignItems: "center",
                          width: "100%",
                        }}
                      >
                        <div>
                          <Text strong>{shortcut.name}</Text>
                          {shortcut.description && (
                            <Text
                              type="secondary"
                              style={{ display: "block", fontSize: 12 }}
                            >
                              {shortcut.description}
                            </Text>
                          )}
                        </div>
                        <Tag
                          color={editingId === shortcut.id ? "blue" : "default"}
                          style={{
                            fontFamily: "'JetBrains Mono', monospace",
                            fontSize: 12,
                            minWidth: 80,
                            textAlign: "center",
                          }}
                        >
                          {editingId === shortcut.id ? (
                            <Input
                              size="small"
                              value={editKey}
                              onChange={(e) => setEditKey(e.target.value)}
                              onKeyDown={handleKeyDown}
                              onBlur={handleSaveEdit}
                              onClick={(e) => e.stopPropagation()}
                              autoFocus
                              style={{ width: 100 }}
                            />
                          ) : (
                            shortcut.currentKey || shortcut.defaultKey
                          )}
                        </Tag>
                      </div>
                    </List.Item>
                  )}
                />
              </Card>
            </Col>
          ))}
        </Row>

        {editingId && (
          <div
            style={{
              position: "fixed",
              bottom: 20,
              left: "50%",
              transform: "translateX(-50%)",
              background: "var(--background)",
              padding: "12px 24px",
              borderRadius: 8,
              boxShadow: "0 4px 12px rgba(0,0,0,0.3)",
              display: "flex",
              gap: 8,
            }}
          >
            <Text>Press keys for shortcut...</Text>
            <Tag
              style={{
                fontFamily: "'JetBrains Mono', monospace",
                background: "var(--accent-primary)",
                color: "var(--background)",
              }}
            >
              {editKey || "..."}
            </Tag>
            <Button size="small" onClick={handleCancelEdit}>
              Cancel
            </Button>
            <Button
              type="primary"
              size="small"
              icon={<Save size={14} />}
              onClick={handleSaveEdit}
            >
              Save
            </Button>
          </div>
        )}
      </div>
    </Modal>
  );
}

export function useKeyboardShortcuts() {
  const [shortcuts] = useState<KeyboardShortcut[]>(() => {
    const stored = localStorage.getItem("axagent-keyboard-shortcuts");
    if (stored) {
      try {
        const parsed = JSON.parse(stored);
        return DEFAULT_SHORTCUTS.map((def) => {
          const saved = parsed.find((s: KeyboardShortcut) => s.id === def.id);
          return saved ? { ...def, currentKey: saved.currentKey || def.defaultKey } : def;
        });
      } catch {
        return DEFAULT_SHORTCUTS.map((s) => ({ ...s, currentKey: s.defaultKey }));
      }
    }
    return DEFAULT_SHORTCUTS.map((s) => ({ ...s, currentKey: s.defaultKey }));
  });

  const getShortcut = (id: string): string | undefined => {
    const shortcut = shortcuts.find((s) => s.id === id);
    return shortcut?.currentKey || shortcut?.defaultKey;
  };

  return { shortcuts, getShortcut };
}
