import { invoke, isTauri, listen, type UnlistenFn } from "@/lib/invoke";
import { useProviderStore, useSettingsStore } from "@/stores";
import { useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import { ModelIcon } from "@lobehub/icons";
import { Input, theme, Tooltip, Typography } from "antd";
import {
  ArrowDownCircle,
  ArrowRight,
  Blocks,
  BookOpen,
  Braces,
  Calculator,
  ChevronDown,
  Copy,
  FileSearch,
  Globe,
  Languages,
  Loader2,
  MemoryStick,
  MessageSquare,
  MessageSquarePlus,
  Replace,
  Search,
  Settings,
  Slash,
  X,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const QUICKBAR_CONV_KEY = "axagent_quickbar_conv_id";
const QUICKBAR_RECENT_KEY = "axagent_quickbar_recent";

/* ── Type system ──────────────────────────────────────────────────── */

type CommandType =
  | "chat"
  | "agent"
  | "url"
  | "search"
  | "wiki"
  | "calc"
  | "model"
  | "new"
  | "continue"
  | "memory"
  | "files"
  | "translate"
  | "summarize"
  | "code"
  | "settings"
  | "gateway";

interface CommandDef {
  key: CommandType;
  labelKey: string;
  descKey: string;
  icon: React.ReactNode;
  category: "ai" | "knowledge" | "web" | "tools" | "system";
  needsBody: boolean;
  needsModel: boolean;
  color: string;
}

interface CategoryDef {
  key: string;
  labelKey: string;
  borderColor: string;
}

/* ── Command definitions ──────────────────────────────────────────── */

const COMMAND_DEFS: (Omit<CommandDef, "labelKey" | "descKey"> & { labelKey: string; descKey: string })[] = [
  {
    key: "chat",
    labelKey: "quickbar.command.chat",
    descKey: "quickbar.command.chatDesc",
    icon: <MessageSquare size={18} />,
    category: "ai",
    needsBody: true,
    needsModel: true,
    color: "#1677ff",
  },
  {
    key: "agent",
    labelKey: "quickbar.command.agent",
    descKey: "quickbar.command.agentDesc",
    icon: <Zap size={18} />,
    category: "ai",
    needsBody: true,
    needsModel: true,
    color: "#722ed1",
  },
  {
    key: "new",
    labelKey: "quickbar.command.new",
    descKey: "quickbar.command.newDesc",
    icon: <MessageSquarePlus size={18} />,
    category: "ai",
    needsBody: false,
    needsModel: false,
    color: "#1677ff",
  },
  {
    key: "continue",
    labelKey: "quickbar.command.continue",
    descKey: "quickbar.command.continueDesc",
    icon: <Blocks size={18} />,
    category: "ai",
    needsBody: false,
    needsModel: false,
    color: "#1677ff",
  },
  {
    key: "search",
    labelKey: "quickbar.command.search",
    descKey: "quickbar.command.searchDesc",
    icon: <Search size={18} />,
    category: "knowledge",
    needsBody: true,
    needsModel: false,
    color: "#52c41a",
  },
  {
    key: "wiki",
    labelKey: "quickbar.command.wiki",
    descKey: "quickbar.command.wikiDesc",
    icon: <BookOpen size={18} />,
    category: "knowledge",
    needsBody: true,
    needsModel: false,
    color: "#52c41a",
  },
  {
    key: "memory",
    labelKey: "quickbar.command.memory",
    descKey: "quickbar.command.memoryDesc",
    icon: <MemoryStick size={18} />,
    category: "knowledge",
    needsBody: true,
    needsModel: false,
    color: "#52c41a",
  },
  {
    key: "files",
    labelKey: "quickbar.command.files",
    descKey: "quickbar.command.filesDesc",
    icon: <FileSearch size={18} />,
    category: "knowledge",
    needsBody: true,
    needsModel: false,
    color: "#52c41a",
  },
  {
    key: "url",
    labelKey: "quickbar.command.url",
    descKey: "quickbar.command.urlDesc",
    icon: <Globe size={18} />,
    category: "web",
    needsBody: true,
    needsModel: true,
    color: "#fa8c16",
  },
  {
    key: "summarize",
    labelKey: "quickbar.command.summarize",
    descKey: "quickbar.command.summarizeDesc",
    icon: <Replace size={18} />,
    category: "web",
    needsBody: true,
    needsModel: true,
    color: "#fa8c16",
  },
  {
    key: "translate",
    labelKey: "quickbar.command.translate",
    descKey: "quickbar.command.translateDesc",
    icon: <Languages size={18} />,
    category: "web",
    needsBody: true,
    needsModel: true,
    color: "#fa8c16",
  },
  {
    key: "calc",
    labelKey: "quickbar.command.calc",
    descKey: "quickbar.command.calcDesc",
    icon: <Calculator size={18} />,
    category: "tools",
    needsBody: true,
    needsModel: false,
    color: "#eb2f96",
  },
  {
    key: "code",
    labelKey: "quickbar.command.code",
    descKey: "quickbar.command.codeDesc",
    icon: <Braces size={18} />,
    category: "tools",
    needsBody: true,
    needsModel: false,
    color: "#eb2f96",
  },
  {
    key: "model",
    labelKey: "quickbar.command.model",
    descKey: "quickbar.command.modelDesc",
    icon: <ChevronDown size={18} />,
    category: "tools",
    needsBody: false,
    needsModel: false,
    color: "#eb2f96",
  },
  {
    key: "settings",
    labelKey: "quickbar.command.settings",
    descKey: "quickbar.command.settingsDesc",
    icon: <Settings size={18} />,
    category: "system",
    needsBody: false,
    needsModel: false,
    color: "#8c8c8c",
  },
  {
    key: "gateway",
    labelKey: "quickbar.command.gateway",
    descKey: "quickbar.command.gatewayDesc",
    icon: <ArrowDownCircle size={18} />,
    category: "system",
    needsBody: false,
    needsModel: false,
    color: "#8c8c8c",
  },
];

const CATEGORY_GROUPS: CategoryDef[] = [
  { key: "ai", labelKey: "quickbar.category.ai", borderColor: "#1677ff" },
  { key: "knowledge", labelKey: "quickbar.category.knowledge", borderColor: "#52c41a" },
  { key: "web", labelKey: "quickbar.category.web", borderColor: "#fa8c16" },
  { key: "tools", labelKey: "quickbar.category.tools", borderColor: "#eb2f96" },
  { key: "system", labelKey: "quickbar.category.system", borderColor: "#8c8c8c" },
];

/* ── Smart parsing ────────────────────────────────────────────────── */

function isUrl(text: string): boolean {
  return /^https?:\/\/\S+$/i.test(text.trim());
}

function isCalcExpr(text: string): boolean {
  const t = text.trim();
  return /^[\d\s+\-*/().%^]+$/.test(t) && /[\d]/.test(t) && /[+\-*/]/.test(t);
}

function parseCommand(raw: string, commands: CommandDef[]): { command: CommandType | null; body: string } {
  const trimmed = raw.trim();
  const match = trimmed.match(/^\/(\w+)\s*(.*)$/s);
  if (match) {
    const cmd = match[1].toLowerCase();
    if (commands.some((c) => c.key === cmd)) {
      return { command: cmd as CommandType, body: match[2].trim() };
    }
  }
  if (isUrl(trimmed)) { return { command: "url", body: trimmed }; }
  if (trimmed.startsWith(">")) { return { command: "agent", body: trimmed.slice(1).trim() }; }
  if (isCalcExpr(trimmed)) { return { command: "calc", body: trimmed }; }
  if (trimmed.length > 0 && !trimmed.startsWith("/")) { return { command: "chat", body: trimmed }; }
  return { command: null, body: trimmed };
}

/* ── Recent items ─────────────────────────────────────────────────── */

function loadRecent(): string[] {
  try {
    const raw = localStorage.getItem(QUICKBAR_RECENT_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveRecent(items: string[]) {
  try {
    localStorage.setItem(QUICKBAR_RECENT_KEY, JSON.stringify(items.slice(0, 5)));
  } catch { /* noop */ }
}

function pushRecent(query: string) {
  const items = loadRecent().filter((i) => i !== query);
  items.unshift(query);
  saveRecent(items);
}

/* ── Component ────────────────────────────────────────────────────── */

export function QuickBarPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const selectedWikiId = useLlmWikiStore((s) => s.selectedWikiId);

  /* Resolve i18n labels into CommandDef */
  const COMMANDS: CommandDef[] = useMemo(
    () => COMMAND_DEFS.map((d) => ({ ...d, labelKey: t(d.labelKey), descKey: t(d.descKey) }) as unknown as CommandDef),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState("");
  const [convId, setConvId] = useState<string | null>(() => localStorage.getItem(QUICKBAR_CONV_KEY));
  const [activeCommand, setActiveCommand] = useState<CommandType | null>(null);
  const [recentItems, setRecentItems] = useState<string[]>(loadRecent);
  const [showCommands, setShowCommands] = useState(false);
  const [showModelList, setShowModelList] = useState(false);
  const [copied, setCopied] = useState(false);
  const [commandMode, setCommandMode] = useState(false);
  const [selectedCmd, setSelectedCmd] = useState(0);

  const inputRef = useRef<any>(null);
  const unlistenRef = useRef<UnlistenFn[]>([]);

  const settings = useSettingsStore((s) => s.settings);
  const activeProviderId = settings.default_provider_id;
  const activeModelId = settings.default_model_id;
  const providers = useProviderStore((s) => s.providers);
  const currentProvider = useMemo(() => providers.find((p) => p.id === activeProviderId), [
    providers,
    activeProviderId,
  ]);
  const currentModels = useMemo(() => currentProvider?.models.filter((m) => m.enabled) ?? [], [currentProvider]);

  /* Command lookup helpers */
  const getCommand = useCallback((key: CommandType) => COMMANDS.find((c) => c.key === key), [COMMANDS]);

  /* ── Lifecycle ──────────────────────────────────────────────────── */

  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 150);
  }, []);

  useEffect(() => {
    if (input.trimStart().startsWith("/") && commandMode) {
      setShowCommands(true);
      setSelectedCmd(0);
    } else {
      setShowCommands(false);
    }
  }, [input, commandMode]);

  const ensureConversation = useCallback(async (): Promise<string> => {
    if (convId) { return convId; }
    const conversation = await invoke<{ id: string }>("create_conversation", { title: "QuickBar" });
    setConvId(conversation.id);
    localStorage.setItem(QUICKBAR_CONV_KEY, conversation.id);
    return conversation.id;
  }, [convId]);

  const cleanupListeners = useCallback(() => {
    for (const fn of unlistenRef.current) { fn(); }
    unlistenRef.current = [];
  }, []);

  useEffect(() => () => cleanupListeners(), [cleanupListeners]);

  const handleHide = useCallback(async () => {
    if (isTauri()) { await invoke("hide_quickbar"); }
  }, []);

  /* ── Keyboard global ─────────────────────────────────────────────── */

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (showCommands) {
          setShowCommands(false);
          return;
        }
        if (showModelList) {
          setShowModelList(false);
          return;
        }
        if (commandMode) {
          setCommandMode(false);
          setInput("");
          setActiveCommand(null);
          return;
        }
        handleHide();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handleHide, showCommands, showModelList, commandMode]);

  /* ── Stream lifecycle ────────────────────────────────────────────── */

  const startStream = useCallback(async (op: () => Promise<void>) => {
    setLoading(true);
    setResult("");
    try {
      await op();
      let text = "";
      cleanupListeners();
      const u1 = await listen<{ conversationId: string; assistantMessageId: string; text: string }>(
        "agent-stream-text",
        (event) => {
          text += event.payload.text;
          setResult(text);
        },
      );
      const u2 = await listen("agent-done", () => setLoading(false));
      const u3 = await listen<{ message: string }>("agent-error", (event) => {
        setResult(`${t("quickbar.result.error")}: ${event.payload.message}`);
        setLoading(false);
      });
      unlistenRef.current = [u1, u2, u3];
    } catch (e) {
      setResult(`${t("quickbar.result.error")}: ${String(e)}`);
      setLoading(false);
    }
  }, [cleanupListeners, t]);

  /* ── Command executors ───────────────────────────────────────────── */

  const runChat = (body: string) =>
    startStream(async () => {
      const cid = await ensureConversation();
      await invoke("send_message", {
        conversationId: cid,
        content: body,
        providerId: activeProviderId,
        modelId: activeModelId,
      });
    });

  const runAgent = (body: string) =>
    startStream(async () => {
      const cid = await ensureConversation();
      await invoke("agent_query", {
        request: { conversationId: cid, input: body, providerId: activeProviderId, model_id: activeModelId },
      }, 0);
    });

  const runUrl = (url: string) =>
    startStream(async () => {
      const cid = await ensureConversation();
      await invoke("agent_query", {
        request: {
          conversationId: cid,
          input: `Fetch the content from this URL and summarize it concisely: ${url}`,
          providerId: activeProviderId,
          model_id: activeModelId,
        },
      }, 0);
    });

  const runSummarizeUrl = (url: string) =>
    startStream(async () => {
      const cid = await ensureConversation();
      await invoke("agent_query", {
        request: {
          conversationId: cid,
          input: `Summarize the core content of this web page in 3-5 sentences: ${url}`,
          providerId: activeProviderId,
          model_id: activeModelId,
        },
      }, 0);
    });

  const runTranslate = (text: string) =>
    startStream(async () => {
      const cid = await ensureConversation();
      await invoke("agent_query", {
        request: {
          conversationId: cid,
          input: `Translate the following content to Chinese:\n\n${text}`,
          providerId: activeProviderId,
          model_id: activeModelId,
        },
      }, 0);
    });

  const runSearch = async (body: string) => {
    setLoading(true);
    setResult("");
    try {
      const results = await invoke<Array<{ content: string; score: number; title: string }>>("search_knowledge_base", {
        query: body,
        limit: 5,
      });
      if (!results || results.length === 0) {
        setResult(t("quickbar.result.noKnowledge"));
        setLoading(false);
        return;
      }
      setResult(
        results.map((r) => `**${r.title}** (${(r.score * 100).toFixed(0)}%)\n${r.content}`).join("\n\n---\n\n"),
      );
    } catch (e) {
      setResult(`${t("quickbar.result.searchFailed")}: ${String(e)}`);
    }
    setLoading(false);
  };

  const runMemorySearch = async (body: string) => {
    setLoading(true);
    setResult("");
    try {
      const results = await invoke<Array<{ content: string; score: number; title: string }>>("search_knowledge_base", {
        query: body,
        limit: 5,
      });
      if (!results || results.length === 0) {
        setResult(t("quickbar.result.noMemory"));
        setLoading(false);
        return;
      }
      setResult(
        results.map((r) => `**${r.title}** (${(r.score * 100).toFixed(0)}%)\n${r.content}`).join("\n\n---\n\n"),
      );
    } catch (e) {
      setResult(`${t("quickbar.result.searchFailed")}: ${String(e)}`);
    }
    setLoading(false);
  };

  const runWiki = async (body: string) => {
    if (!body.trim()) { return; }
    setLoading(true);
    setResult("");
    try {
      if (!selectedWikiId) {
        setResult(t("quickbar.result.noWikiSelected"));
        setLoading(false);
        return;
      }
      const safeTitle = `QuickBar - ${new Date().toLocaleString()}`;
      await invoke("llm_wiki_ingest", {
        wikiId: selectedWikiId,
        sourceType: "markdown",
        path: `quickbar/${safeTitle.replace(/[/\\:*?"<>|]/g, "_")}.md`,
        title: safeTitle,
      });
      setResult(`✅ ${t("quickbar.result.savedWiki")}`);
    } catch (e) {
      setResult(`${t("quickbar.result.saveWikiFailed")}: ${String(e)}`);
    }
    setLoading(false);
  };

  const runCalc = async (expr: string) => {
    try {
      const sanitized = expr.replace(/[^0-9+\-*/().%\s]/g, "");
      const value = Function(`"use strict"; return (${sanitized})`)();
      if (value === Infinity || value === -Infinity) { throw new Error("Division by zero"); }
      setResult(`${expr.trim()} = ${Number.isInteger(value) ? value : value.toFixed(6)}`);
    } catch {
      await runChat(`${expr} = ?`);
    }
  };

  const runCode = async (code: string) =>
    startStream(async () => {
      const cid = await ensureConversation();
      await invoke("send_message", {
        conversationId: cid,
        content: `Execute the following code in a sandbox and return the result:\n\`\`\`\n${code}\n\`\`\``,
        providerId: activeProviderId,
        modelId: activeModelId,
      });
    });

  const runModelSwitch = (modelId: string) => {
    const store = useSettingsStore.getState();
    store.saveSettings({ default_model_id: modelId });
    setShowModelList(false);
    setResult(`✅ ${t("quickbar.result.modelSwitched")}`);
    setTimeout(() => setResult(""), 1500);
  };

  const runNewConversation = useCallback(async () => {
    setLoading(true);
    try {
      const conversation = await invoke<{ id: string }>("create_conversation", { title: "QuickBar" });
      setConvId(conversation.id);
      localStorage.setItem(QUICKBAR_CONV_KEY, conversation.id);
      setResult(`✅ ${t("quickbar.result.newConversation")}`);
      setActiveCommand(null);
    } catch (e) {
      setResult(`${t("quickbar.result.createFailed")}: ${String(e)}`);
    }
    setLoading(false);
  }, [t]);

  /* ── Tile click handler ──────────────────────────────────────────── */

  const handleTileClick = useCallback(async (cmd: CommandDef) => {
    if (cmd.needsBody) {
      setCommandMode(true);
      setActiveCommand(cmd.key);
      setInput(`/${cmd.key} `);
      setResult("");
      setTimeout(() => inputRef.current?.focus(), 50);
    } else {
      switch (cmd.key) {
        case "new":
          await runNewConversation();
          break;
        case "continue":
          break;
        case "model":
          setShowModelList(true);
          break;
        case "settings":
          window.open("/settings", "_blank", "noopener,noreferrer");
          break;
        case "gateway":
          window.open("/gateway", "_blank", "noopener,noreferrer");
          break;
      }
    }
  }, [runNewConversation]);

  /* ── Submit handler ───────────────────────────────────────────────── */

  const handleSubmit = useCallback(async () => {
    const { command, body } = parseCommand(input, COMMANDS);
    if (!body && command !== "new" && command !== "model") { return; }

    setShowCommands(false);
    setActiveCommand(command);
    if (body) {
      pushRecent(input.trim());
      setRecentItems(loadRecent());
    }

    switch (command) {
      case "chat":
        await runChat(body);
        break;
      case "agent":
        await runAgent(body);
        break;
      case "url":
        await runUrl(body);
        break;
      case "search":
        await runSearch(body);
        break;
      case "wiki":
        await runWiki(body);
        break;
      case "calc":
        await runCalc(body);
        break;
      case "code":
        await runCode(body);
        break;
      case "memory":
        await runMemorySearch(body);
        break;
      case "translate":
        await runTranslate(body);
        break;
      case "summarize":
        await runSummarizeUrl(body);
        break;
      case "files":
        await runSearch(body);
        break;
      case "new":
        await runNewConversation();
        break;
      case "model":
        setShowModelList(true);
        break;
      case "settings":
        window.open("/settings", "_blank", "noopener,noreferrer");
        break;
      case "gateway":
        window.open("/gateway", "_blank", "noopener,noreferrer");
        break;
      default:
        break;
    }
  }, [
    input,
    COMMANDS,
    runChat,
    runAgent,
    runUrl,
    runSearch,
    runWiki,
    runCalc,
    runTranslate,
    runSummarizeUrl,
    runCode,
    runMemorySearch,
    runNewConversation,
  ]);

  /* ── Keyboard in command palette ──────────────────────────────────── */

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (showCommands) {
      const partial = input.trimStart().slice(1).toLowerCase();
      const visible = COMMANDS.filter((c) => c.key.startsWith(partial) || c.labelKey.toLowerCase().includes(partial));
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedCmd((i) => Math.min(i + 1, visible.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedCmd((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        const cmd = visible[selectedCmd];
        if (cmd) {
          setInput(`/${cmd.key} `);
          setActiveCommand(cmd.key);
          setShowCommands(false);
        }
        setTimeout(() => inputRef.current?.focus(), 50);
      }
      return;
    }
    if (!commandMode && e.key === "/" && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      setCommandMode(true);
      setInput("/");
      setTimeout(() => inputRef.current?.focus(), 50);
    }
    if (!commandMode && e.key === "Enter" && !showModelList) {
      e.preventDefault();
      setCommandMode(true);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [showCommands, input, selectedCmd, COMMANDS, commandMode, showModelList]);

  /* ── Misc handlers ────────────────────────────────────────────────── */

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(result);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {}
  }, [result]);

  const handleContinue = useCallback(() => {
    setCommandMode(true);
    setInput(`${result.slice(-500)}\n`);
    setResult("");
    setTimeout(() => inputRef.current?.focus(), 50);
  }, [result]);

  const clearAll = useCallback(() => {
    setInput("");
    setResult("");
    setActiveCommand(null);
    setCommandMode(false);
    setShowCommands(false);
  }, []);

  /* ── Helpers ──────────────────────────────────────────────────────── */

  const borderColor = token.colorBorderSecondary;
  const hasResult = result.length > 0;
  const showClear = input.length > 0 || hasResult;
  const categorizedCommands = useMemo(() => {
    const map: Record<string, CommandDef[]> = {};
    for (const g of CATEGORY_GROUPS) { map[g.key] = COMMANDS.filter((c) => c.category === g.key); }
    return map;
  }, [COMMANDS]);

  /* ── Reusable styles ──────────────────────────────────────────────── */

  const actionBtnStyle = (color: string): React.CSSProperties => ({
    display: "inline-flex",
    alignItems: "center",
    gap: 4,
    background: "none",
    border: "none",
    cursor: "pointer",
    padding: "3px 8px",
    borderRadius: 4,
    fontSize: 11,
    color,
    transition: "background-color 0.15s",
  });

  const actionHover = (bg: string) => (e: React.MouseEvent<HTMLButtonElement>) => {
    e.currentTarget.style.backgroundColor = bg;
  };

  /* ── Render: Tile grid mode ─────────────────────────────────────── */

  const renderTileGrid = () => (
    <div style={{ flex: 1, overflowY: "auto", padding: "12px 14px" }}>
      {CATEGORY_GROUPS.map((group) => {
        const cmds = categorizedCommands[group.key];
        if (!cmds.length) { return null; }
        return (
          <div key={group.key} style={{ marginBottom: 16 }}>
            <div
              style={{
                fontSize: 11,
                fontWeight: 600,
                color: token.colorTextSecondary,
                marginBottom: 8,
                paddingLeft: 2,
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              <span
                style={{
                  width: 3,
                  height: 12,
                  borderRadius: 2,
                  backgroundColor: group.borderColor,
                  display: "inline-block",
                }}
              />
              {t(group.labelKey)}
            </div>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(140px, 1fr))",
                gap: 6,
              }}
            >
              {cmds.map((cmd) => (
                <button
                  key={cmd.key}
                  onClick={() => handleTileClick(cmd)}
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    alignItems: "flex-start",
                    gap: 6,
                    padding: "10px 12px",
                    backgroundColor: token.colorBgElevated,
                    border: `1px solid ${borderColor}`,
                    borderRadius: 8,
                    cursor: "pointer",
                    textAlign: "left" as const,
                    transition: "all 0.15s ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.borderColor = cmd.color;
                    e.currentTarget.style.backgroundColor = `${cmd.color}10`;
                    e.currentTarget.style.transform = "translateY(-1px)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = borderColor;
                    e.currentTarget.style.backgroundColor = token.colorBgElevated;
                    e.currentTarget.style.transform = "none";
                  }}
                >
                  <span style={{ color: cmd.color, display: "flex" }}>{cmd.icon}</span>
                  <div>
                    <div style={{ fontSize: 12, fontWeight: 500, color: token.colorText, lineHeight: 1.4 }}>
                      {cmd.labelKey}
                    </div>
                    <div style={{ fontSize: 10, color: token.colorTextQuaternary, lineHeight: 1.3, marginTop: 1 }}>
                      {cmd.descKey}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );

  /* ── Render: Command input mode ──────────────────────────────────── */

  const activeCmdDef = activeCommand ? getCommand(activeCommand) : null;

  const renderCommandMode = () => (
    <>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "8px 10px",
          flexShrink: 0,
          position: "relative",
        }}
      >
        <span
          onClick={() => {
            setCommandMode(false);
            setInput("");
            setActiveCommand(null);
          }}
          style={{ opacity: 0.4, cursor: "pointer", fontSize: 14, flexShrink: 0, color: token.colorTextSecondary }}
          title={t("quickbar.cmdTile")}
        >
          ←
        </span>
        {activeCmdDef && (
          <span
            style={{
              fontSize: 11,
              fontWeight: 600,
              padding: "1px 6px",
              backgroundColor: `${activeCmdDef.color}15`,
              color: activeCmdDef.color,
              borderRadius: 4,
              flexShrink: 0,
              fontFamily: "var(--code-font-family, monospace)",
            }}
          >
            /{activeCmdDef.key}
          </span>
        )}
        <Input
          ref={inputRef}
          placeholder={showCommands
            ? t("quickbar.selectCommand")
            : activeCmdDef
            ? t("quickbar.inputPlaceholderCommand", { cmdLabel: activeCmdDef.labelKey })
            : t("quickbar.inputPlaceholder")}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onPressEnter={handleSubmit}
          onKeyDown={handleKeyDown}
          variant="borderless"
          disabled={loading}
          style={{ flex: 1, fontSize: 14, backgroundColor: "transparent" }}
        />
        {loading
          ? <Loader2 size={16} className="animate-spin" style={{ opacity: 0.5, flexShrink: 0 }} />
          : showClear && (
            <button
              onClick={clearAll}
              style={{ background: "none", border: "none", cursor: "pointer", padding: 4, opacity: 0.4 }}
            >
              <X size={14} color={token.colorTextSecondary} />
            </button>
          )}
      </div>

      {showCommands && (
        <div
          style={{
            margin: "0 10px",
            padding: "4px 0",
            backgroundColor: token.colorBgElevated,
            border: `1px solid ${borderColor}`,
            borderRadius: 8,
            boxShadow: `0 4px 16px rgba(0,0,0,0.2)`,
            zIndex: 10,
            flexShrink: 0,
            maxHeight: 240,
            overflowY: "auto",
          }}
        >
          {(() => {
            const partial = input.trimStart().slice(1).toLowerCase();
            const visible = COMMANDS.filter((c) =>
              c.key.startsWith(partial) || c.labelKey.toLowerCase().includes(partial)
            );
            return visible.map((cmd, idx) => (
              <div
                key={cmd.key}
                onClick={() => {
                  setInput(`/${cmd.key} `);
                  setActiveCommand(cmd.key);
                  setShowCommands(false);
                }}
                onMouseEnter={() => setSelectedCmd(idx)}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 10,
                  padding: "6px 12px",
                  cursor: "pointer",
                  fontSize: 12,
                  backgroundColor: idx === selectedCmd ? token.colorFillSecondary : "transparent",
                  color: token.colorText,
                  transition: "background-color 0.1s",
                }}
              >
                <span style={{ color: cmd.color, display: "flex" }}>{cmd.icon}</span>
                <span style={{ fontWeight: 500, fontFamily: "var(--code-font-family, monospace)", minWidth: 50 }}>
                  /{cmd.key}
                </span>
                <span style={{ opacity: 0.5 }}>{cmd.labelKey}</span>
                <span style={{ opacity: 0.35, marginLeft: "auto", fontSize: 10 }}>{cmd.descKey}</span>
              </div>
            ));
          })()}
        </div>
      )}

      {!hasResult && !showCommands && !showModelList && (
        <div
          style={{
            display: "flex",
            gap: 10,
            padding: "2px 10px 6px",
            fontSize: 10,
            color: token.colorTextQuaternary,
            flexShrink: 0,
            overflow: "hidden",
            flexWrap: "wrap",
          }}
        >
          {COMMANDS.map((c) => (
            <span
              key={c.key}
              onClick={() => {
                setInput(`/${c.key} `);
                setActiveCommand(c.key);
              }}
              style={{ cursor: "pointer", whiteSpace: "nowrap", opacity: 0.6 }}
              title={`/${c.key} - ${c.descKey}`}
            >
              /{c.key}
            </span>
          ))}
        </div>
      )}
    </>
  );

  /* ── Render: Result area ─────────────────────────────────────────── */

  const renderResult = () => {
    if (!hasResult && !showModelList) { return null; }
    if (showModelList) {
      return (
        <>
          <div style={{ height: 1, backgroundColor: borderColor, flexShrink: 0 }} />
          <div style={{ flex: 1, overflowY: "auto", padding: "6px 0" }}>
            {currentModels.map((m) => (
              <div
                key={m.model_id}
                onClick={() => runModelSwitch(m.model_id)}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "8px 14px",
                  cursor: "pointer",
                  fontSize: 13,
                  backgroundColor: m.model_id === activeModelId ? token.colorFillSecondary : "transparent",
                  color: token.colorText,
                  transition: "background-color 0.1s",
                }}
              >
                <ModelIcon model={m.model_id} size={20} type="avatar" />
                <span style={{ flex: 1 }}>{m.model_id}</span>
                {m.model_id === activeModelId && (
                  <span style={{ fontSize: 10, color: token.colorPrimary }}>{t("quickbar.result.current")}</span>
                )}
              </div>
            ))}
          </div>
        </>
      );
    }
    return (
      <>
        <div style={{ height: 1, backgroundColor: borderColor, flexShrink: 0 }} />
        <div
          style={{
            flex: 1,
            padding: "10px 14px",
            overflowY: "auto",
            fontSize: 13,
            lineHeight: 1.7,
            color: token.colorText,
            maxHeight: "55vh",
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          <Typography.Text style={{ fontSize: 13 }}>{result}</Typography.Text>
        </div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "5px 12px",
            borderTop: `1px solid ${borderColor}`,
            flexShrink: 0,
          }}
        >
          <Tooltip title={copied ? t("quickbar.result.copied") : t("quickbar.result.copy")}>
            <button
              onClick={handleCopy}
              style={actionBtnStyle(copied ? token.colorSuccess : token.colorTextSecondary)}
              onMouseEnter={actionHover(token.colorFillSecondary)}
              onMouseLeave={actionHover("transparent")}
            >
              <Copy size={12} /> {copied ? t("quickbar.result.copied") : t("quickbar.result.copy")}
            </button>
          </Tooltip>
          <Tooltip title={t("quickbar.result.saveWiki")}>
            <button
              onClick={async () => {
                if (!result.trim()) { return; }
                setLoading(true);
                try {
                  if (!selectedWikiId) {
                    setResult((p) => p + "\n\n❌ " + t("quickbar.result.noWikiSelected"));
                    setLoading(false);
                    return;
                  }
                  const safeTitle = `QuickBar - ${new Date().toLocaleString()}`;
                  await invoke("llm_wiki_ingest", {
                    wikiId: selectedWikiId,
                    sourceType: "markdown",
                    path: `quickbar/${safeTitle.replace(/[/\\:*?"<>|]/g, "_")}.md`,
                    title: safeTitle,
                  });
                  setResult((p) => p + `\n\n✅ ${t("quickbar.result.savedWiki")}`);
                } catch (e) {
                  setResult((p) => p + `\n\n❌ ${String(e)}`);
                }
                setLoading(false);
              }}
              style={actionBtnStyle(token.colorTextSecondary)}
              onMouseEnter={actionHover(token.colorFillSecondary)}
              onMouseLeave={actionHover("transparent")}
            >
              <BookOpen size={12} /> {t("quickbar.result.saveWiki")}
            </button>
          </Tooltip>
          <Tooltip title={t("quickbar.result.continueAsk")}>
            <button
              onClick={handleContinue}
              style={actionBtnStyle(token.colorTextSecondary)}
              onMouseEnter={actionHover(token.colorFillSecondary)}
              onMouseLeave={actionHover("transparent")}
            >
              <ArrowRight size={12} /> {t("quickbar.result.continueAsk")}
            </button>
          </Tooltip>
          <div style={{ flex: 1 }} />
        </div>
      </>
    );
  };

  /* ── Main render ─────────────────────────────────────────────────── */

  return (
    <div
      className="ax-page-transition"
      style={{
        height: "100vh",
        display: "flex",
        flexDirection: "column",
        backgroundColor: token.colorBgContainer,
        userSelect: "none",
      }}
    >
      {/* ── Header ──────────────────────────────────────────────────── */}
      <div
        className="ax-titlebar-compact title-bar-drag"
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          paddingLeft: 14,
          paddingRight: 8,
          height: 28,
          flexShrink: 0,
          borderBottom: `1px solid ${borderColor}`,
        }}
      >
        <span style={{ fontSize: 12, fontWeight: 500, color: token.colorTextSecondary }}>
          {t("quickbar.title")}
          {activeCommand ? ` · /${activeCommand}` : ""}
        </span>
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          {activeModelId && (
            <Tooltip title={activeModelId}>
              <button
                className="title-bar-nodrag"
                onClick={() => setShowModelList((v) => !v)}
                style={{ background: "none", border: "none", cursor: "pointer", padding: 2 }}
              >
                <ModelIcon model={activeModelId} size={16} type="avatar" />
              </button>
            </Tooltip>
          )}
          <Tooltip title={commandMode ? t("quickbar.cmdTile") : t("quickbar.cmdLine")}>
            <button
              className="title-bar-nodrag"
              onClick={() => {
                setCommandMode(!commandMode);
                setInput("");
                setActiveCommand(null);
                setShowCommands(false);
              }}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 4,
                opacity: commandMode ? 0.8 : 0.4,
                color: token.colorTextSecondary,
              }}
            >
              {commandMode ? <Slash size={14} /> : <Blocks size={14} />}
            </button>
          </Tooltip>
          <button
            className="title-bar-nodrag"
            onClick={handleHide}
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 4,
              opacity: 0.4,
              color: token.colorTextSecondary,
            }}
          >
            <X size={14} />
          </button>
        </div>
      </div>

      {/* ── Main content ─────────────────────────────────────────────── */}
      {!commandMode
        ? (
          <>
            {recentItems.length > 0 && !hasResult && (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "4px 14px",
                  fontSize: 11,
                  color: token.colorTextQuaternary,
                  flexShrink: 0,
                  overflowX: "auto",
                  whiteSpace: "nowrap",
                }}
              >
                <span style={{ flexShrink: 0 }}>{t("quickbar.recent")}:</span>
                {recentItems.map((item, i) => (
                  <span
                    key={i}
                    onClick={() => {
                      setCommandMode(true);
                      setInput(item);
                      setTimeout(() => inputRef.current?.focus(), 50);
                    }}
                    style={{
                      cursor: "pointer",
                      opacity: 0.65,
                      maxWidth: 140,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                    title={item}
                  >
                    {item.length > 30 ? item.slice(0, 30) + "…" : item}
                  </span>
                ))}
                <span
                  onClick={() => {
                    saveRecent([]);
                    setRecentItems([]);
                  }}
                  style={{ cursor: "pointer", opacity: 0.35, marginLeft: "auto", flexShrink: 0 }}
                >
                  {t("quickbar.clearRecent")}
                </span>
              </div>
            )}
            {renderTileGrid()}
            {!hasResult && (
              <div
                style={{
                  padding: "6px 14px 10px",
                  fontSize: 10,
                  color: token.colorTextQuaternary,
                  textAlign: "center",
                  borderTop: `1px solid ${borderColor}`,
                  flexShrink: 0,
                }}
              >
                {t("quickbar.hintTileMode")}
              </div>
            )}
          </>
        )
        : (
          renderCommandMode()
        )}

      {!commandMode && !hasResult && recentItems.length === 0 && (
        <div
          style={{
            position: "absolute",
            bottom: 40,
            left: 0,
            right: 0,
            textAlign: "center",
            opacity: 0.08,
            pointerEvents: "none",
          }}
        >
          <div style={{ fontSize: 36 }}>▸</div>
        </div>
      )}

      {renderResult()}
    </div>
  );
}
