import { CHAT_ICON_COLORS } from "@/lib/iconColors";
import { executeSkillAction } from "@/lib/skillActionExecutor";
import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore, useUIStore } from "@/stores";
import { Input, List, Modal, Tag, theme, Typography } from "antd";
import { MessageSquare, Network, PanelLeftClose, Plus, Search, Settings, Sparkles } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

export interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
}

export interface Command {
  id: string;
  label: string;
  icon: React.ReactNode;
  shortcut?: string;
  category: string;
  action: () => void;
}

// ─── 动态命令注册表 ───
const commandRegistry: Command[] = [];

export function registerCommand(cmd: Command) {
  if (!commandRegistry.find((c) => c.id === cmd.id)) {
    commandRegistry.push(cmd);
  }
}

export function unregisterCommand(id: string) {
  const idx = commandRegistry.findIndex((c) => c.id === id);
  if (idx !== -1) commandRegistry.splice(idx, 1);
}

// ─── 使用频率持久化 ───
const USE_COUNT_KEY = "axagent:cmd-use-count";
function loadUseCounts(): Map<string, number> {
  try {
    const raw = localStorage.getItem(USE_COUNT_KEY);
    if (raw) return new Map(JSON.parse(raw));
  } catch { /* ignore */ }
  return new Map();
}
function saveUseCounts(counts: Map<string, number>) {
  try {
    localStorage.setItem(USE_COUNT_KEY, JSON.stringify([...counts]));
  } catch { /* ignore */ }
}

// ─── 简易模糊匹配评分 ───
function fuzzyScore(text: string, query: string): number {
  const lower = text.toLowerCase();
  const q = query.toLowerCase();
  if (lower === q) return 100;
  if (lower.startsWith(q)) return 80;
  if (lower.includes(q)) return 50;

  // 字符序列匹配（abc 匹配 "a.*b.*c"）
  let qi = 0;
  let score = 0;
  for (let i = 0; i < lower.length && qi < q.length; i++) {
    if (lower[i] === q[qi]) {
      score += 10 - qi * 2; // 越靠前的匹配得分越高
      qi++;
    }
  }
  return qi === q.length ? score : 0;
}

export default function CommandPalette({ open, onClose }: CommandPaletteProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const [useCounts, setUseCounts] = useState<Map<string, number>>(() => loadUseCounts());

  const navigate = useNavigate();
  const setSettingsSection = useUIStore((s) => s.setSettingsSection);
  const toggleSidebar = useUIStore((s) => s.toggleSidebar);

  // 基础命令 + 注册的动态命令
  const commands = useMemo<Command[]>(() => {
    const nav = t("commandPalette.navigation");
    const actions = t("commandPalette.actions");
    const settings = t("commandPalette.settings");

    const builtin: Command[] = [
      {
        id: "go-chat",
        label: t("commandPalette.goToChat"),
        icon: <MessageSquare size={16} color={CHAT_ICON_COLORS.MessageSquare} />,
        category: nav,
        action: () => { navigate("/"); onClose(); },
      },
      {
        id: "go-settings",
        label: t("commandPalette.goToSettings"),
        icon: <Settings size={16} color={CHAT_ICON_COLORS.Settings} />,
        shortcut: "⌘,",
        category: nav,
        action: () => { navigate("/settings"); onClose(); },
      },
      {
        id: "go-gateway",
        label: t("commandPalette.goToGateway"),
        icon: <Network size={16} color={CHAT_ICON_COLORS.Network} />,
        category: nav,
        action: () => { navigate("/gateway"); onClose(); },
      },
      {
        id: "go-skills",
        label: t("commandPalette.goToSkills"),
        icon: <Sparkles size={16} color={CHAT_ICON_COLORS.Sparkles} />,
        category: nav,
        action: () => { navigate("/skills"); onClose(); },
      },
      {
        id: "new-conversation",
        label: t("commandPalette.newConversation"),
        icon: <Plus size={16} color={CHAT_ICON_COLORS.Plus} />,
        shortcut: "⌘N",
        category: actions,
        action: () => { navigate("/"); onClose(); },
      },
      {
        id: "toggle-sidebar",
        label: t("commandPalette.toggleSidebar"),
        icon: <PanelLeftClose size={16} color={CHAT_ICON_COLORS.PanelLeftClose} />,
        category: actions,
        action: () => { toggleSidebar(); onClose(); },
      },
      {
        id: "search-conversations",
        label: t("commandPalette.searchConversations"),
        icon: <Search size={16} color={CHAT_ICON_COLORS.Search} />,
        shortcut: "⌘F",
        category: actions,
        action: () => { navigate("/"); onClose(); },
      },
      {
        id: "settings-search",
        label: `${t("commandPalette.goToSettings")} → ${t("settings.searchProviders.title")}`,
        icon: <Settings size={16} color={CHAT_ICON_COLORS.Settings} />,
        category: settings,
        action: () => { navigate("/settings"); onClose(); },
      },
      {
        id: "settings-mcp",
        label: `${t("commandPalette.goToSettings")} → ${t("settings.mcpServers.title")}`,
        icon: <Settings size={16} color={CHAT_ICON_COLORS.Settings} />,
        category: settings,
        action: () => { navigate("/settings"); onClose(); },
      },
    ];

    // 合并动态注册的命令（去重）
    const ids = new Set(builtin.map((c) => c.id));
    const extra = commandRegistry.filter((c) => !ids.has(c.id));
    return [...builtin, ...extra];
  }, [t, navigate, setSettingsSection, toggleSidebar, onClose]);

  const filtered = useMemo(() => {
    if (!query.trim()) {
      // 无搜索时按使用频率降序排列
      return [...commands].sort((a, b) => {
        const ua = useCounts.get(a.id) ?? 0;
        const ub = useCounts.get(b.id) ?? 0;
        return ub - ua;
      });
    }
    const q = query.trim();
    const scored = commands
      .map((c) => ({
        cmd: c,
        score: fuzzyScore(c.label, q) + fuzzyScore(c.category, q) * 0.5,
      }))
      .filter((s) => s.score > 0)
      .sort((a, b) => b.score - a.score);
    return scored.map((s) => s.cmd);
  }, [commands, query, useCounts]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  useEffect(() => {
    if (!open) {
      setQuery("");
      setActiveIndex(0);
    }
  }, [open]);

  // 注册技能扩展命令
  const skillCommands = useSkillExtensionStore((s) => s.commands);
  useEffect(() => {
    const registeredIds: string[] = [];
    for (const cmd of skillCommands) {
      const cmdId = `skill:${cmd.skillName}:${cmd.id}`;
      const IconComp = cmd.icon ? resolveIconComponent(cmd.icon) : Settings;
      registerCommand({
        id: cmdId,
        label: cmd.label,
        icon: <IconComp size={16} />,
        shortcut: cmd.shortcut,
        category: cmd.skillName,
        action: () => {
          executeSkillAction(cmd.action, navigate);
          onClose();
        },
      });
      registeredIds.push(cmdId);
    }
    return () => {
      for (const id of registeredIds) {
        unregisterCommand(id);
      }
    };
  }, [skillCommands, navigate, onClose]);

  // 执行命令时记录使用次数
  const executeCommand = useCallback((cmd: Command) => {
    setUseCounts((prev) => {
      const next = new Map(prev);
      next.set(cmd.id, (next.get(cmd.id) ?? 0) + 1);
      saveUseCounts(next);
      return next;
    });
    cmd.action();
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((prev) => (prev + 1) % filtered.length);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex((prev) => (prev - 1 + filtered.length) % filtered.length);
      } else if (e.key === "Enter" && filtered.length > 0) {
        e.preventDefault();
        executeCommand(filtered[activeIndex]);
      }
    },
    [filtered, activeIndex, executeCommand],
  );

  // Group commands by category for display
  const grouped = useMemo(() => {
    const groups: Record<string, Command[]> = {};
    for (const cmd of filtered) {
      if (!groups[cmd.category]) { groups[cmd.category] = []; }
      groups[cmd.category].push(cmd);
    }
    return groups;
  }, [filtered]);

  let flatIndex = 0;

  return (
    <Modal
      open={open}
      onCancel={onClose}
      mask={{ enabled: true, blur: true }}
      footer={null}
      closable={false}
      centered
      width={600}
      styles={{ body: { padding: 0 } }}
    >
      <div onKeyDown={handleKeyDown}>
        <Input
          prefix={<Search size={16} color={CHAT_ICON_COLORS.Search} />}
          placeholder={t("commandPalette.placeholder")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          variant="borderless"
          size="large"
          autoFocus
          style={{ padding: "12px 16px" }}
        />
        <div
          ref={listRef}
          data-os-scrollbar
          style={{
            maxHeight: 400,
            overflowY: "auto",
            borderTop: "1px solid var(--border-color)",
          }}
        >
          {Object.entries(grouped).map(([category, cmds]) => (
            <div key={category}>
              <Typography.Text
                type="secondary"
                style={{
                  display: "block",
                  padding: "8px 16px 4px",
                  fontSize: 12,
                  fontWeight: 500,
                }}
              >
                {category}
              </Typography.Text>
              <List
                dataSource={cmds}
                renderItem={(cmd) => {
                  const idx = flatIndex++;
                  const isActive = idx === activeIndex;
                  return (
                    <List.Item
                      key={cmd.id}
                      onClick={() => executeCommand(cmd)}
                      style={{
                        cursor: "pointer",
                        padding: "8px 16px",
                        backgroundColor: isActive ? token.colorBgTextHover : undefined,
                      }}
                    >
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          width: "100%",
                          gap: 8,
                        }}
                      >
                        <span style={{ fontSize: 16 }}>{cmd.icon}</span>
                        <span style={{ flex: 1 }}>{cmd.label}</span>
                        {cmd.shortcut && <Tag style={{ margin: 0 }}>{cmd.shortcut}</Tag>}
                      </div>
                    </List.Item>
                  );
                }}
              />
            </div>
          ))}
        </div>
      </div>
    </Modal>
  );
}
