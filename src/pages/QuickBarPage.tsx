import { invoke, isTauri, listen, type UnlistenFn } from "@/lib/invoke";
import { useProviderStore, useSettingsStore } from "@/stores";
import { Input, theme, Tooltip, Typography } from "antd";
import {
  Brain,
  ChevronDown,
  Copy,
  Globe,
  Loader2,
  ArrowRight,
  BookOpen,
  MessageSquare,
  Search,
  X,
  Zap,
} from "lucide-react";
import { ModelIcon } from "@lobehub/icons";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

const QUICKBAR_CONV_KEY = "axagent_quickbar_conv_id";
const QUICKBAR_RECENT_KEY = "axagent_quickbar_recent";

type CommandType = "chat" | "agent" | "url" | "search" | "wiki" | "calc" | "model";

interface CommandDef {
  key: CommandType;
  icon: React.ReactNode;
  label: string;
  hint: string;
}

const COMMANDS: CommandDef[] = [
  { key: "chat", icon: <MessageSquare size={14} />, label: "chat", hint: "快速问答" },
  { key: "agent", icon: <Zap size={14} />, label: "agent", hint: "执行 Agent 任务" },
  { key: "url", icon: <Globe size={14} />, label: "url", hint: "抓取网页内容" },
  { key: "search", icon: <Search size={14} />, label: "search", hint: "搜索知识库" },
  { key: "wiki", icon: <BookOpen size={14} />, label: "wiki", hint: "存入知识库" },
  { key: "calc", icon: <Brain size={14} />, label: "calc", hint: "数学计算" },
  { key: "model", icon: <ChevronDown size={14} />, label: "model", hint: "切换模型" },
];

function isUrl(text: string): boolean {
  return /^https?:\/\/\S+$/i.test(text.trim());
}

function isCalcExpr(text: string): boolean {
  const t = text.trim();
  return /^[\d\s+\-*/().%^]+$/.test(t) && /[\d]/.test(t) && /[+\-*/]/.test(t);
}

function parseCommand(raw: string): { command: CommandType | null; body: string } {
  const trimmed = raw.trim();
  const match = trimmed.match(/^\/(\w+)\s*(.*)$/s);
  if (match) {
    const cmd = match[1].toLowerCase();
    const validCommands = COMMANDS.map((c) => c.key);
    if (validCommands.includes(cmd as CommandType)) {
      return { command: cmd as CommandType, body: match[2].trim() };
    }
  }
  // Smart detection (no / prefix)
  if (isUrl(trimmed)) return { command: "url", body: trimmed };
  if (trimmed.startsWith(">")) return { command: "agent", body: trimmed.slice(1).trim() };
  if (isCalcExpr(trimmed)) return { command: "calc", body: trimmed };
  return { command: null, body: trimmed }; // null = default chat
}

function resolveCommand(input: string): { command: CommandType; body: string } {
  const { command, body } = parseCommand(input);
  return { command: command ?? "chat", body };
}

/** Recent items — max 3, persisted in localStorage */
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
    localStorage.setItem(QUICKBAR_RECENT_KEY, JSON.stringify(items.slice(0, 3)));
  } catch { /* noop */ }
}

function pushRecent(query: string) {
  const items = loadRecent().filter((i) => i !== query);
  items.unshift(query);
  saveRecent(items);
}

export function QuickBarPage() {
  const { token } = theme.useToken();
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState("");
  const [convId, setConvId] = useState<string | null>(
    () => localStorage.getItem(QUICKBAR_CONV_KEY),
  );
  const [mode, setMode] = useState<CommandType | null>(null);
  const [recentItems, setRecentItems] = useState<string[]>(loadRecent);
  const [showCommands, setShowCommands] = useState(false);
  const [selectedCmd, setSelectedCmd] = useState(0);
  const [copied, setCopied] = useState(false);
  const [showModelList, setShowModelList] = useState(false);

  const inputRef = useRef<any>(null);
  const unlistenRef = useRef<UnlistenFn[]>([]);
  const resultRef = useRef<HTMLDivElement>(null);

  const settings = useSettingsStore((s) => s.settings);
  const activeProviderId = settings.default_provider_id;
  const activeModelId = settings.default_model_id;
  const providers = useProviderStore((s) => s.providers);
  const currentProvider = useMemo(
    () => providers.find((p) => p.id === activeProviderId),
    [providers, activeProviderId],
  );
  const currentModels = useMemo(
    () => currentProvider?.models.filter((m) => m.enabled) ?? [],
    [currentProvider],
  );

  // Auto-detect /cmd prefix to show command palette
  useEffect(() => {
    if (input.trimStart().startsWith("/") && !input.includes(" ")) {
      setShowCommands(true);
      setSelectedCmd(0);
    } else {
      setShowCommands(false);
    }
  }, [input]);

  // Resolve mode when user submits
  const resolvedMode = useMemo(() => {
    if (!input.trim()) return null;
    return resolveCommand(input).command;
  }, [input]);

  // Focus input on mount
  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 100);
  }, []);

  // Ensure conversation exists
  const ensureConversation = useCallback(async (): Promise<string> => {
    if (convId) return convId;
    const conversation = await invoke<{ id: string }>("create_conversation", {
      title: "QuickBar",
    });
    setConvId(conversation.id);
    localStorage.setItem(QUICKBAR_CONV_KEY, conversation.id);
    return conversation.id;
  }, [convId]);

  // Cleanup stream listeners
  const cleanupListeners = useCallback(() => {
    for (const fn of unlistenRef.current) fn();
    unlistenRef.current = [];
  }, []);

  useEffect(() => () => cleanupListeners(), [cleanupListeners]);

  // Hide window
  const handleHide = useCallback(async () => {
    if (isTauri()) await invoke("hide_quickbar");
  }, []);

  // Escape key
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (showCommands) { setShowCommands(false); return; }
        if (showModelList) { setShowModelList(false); return; }
        handleHide();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handleHide, showCommands, showModelList]);

  // ── Stream helpers ────────────────────────────────────────────────
  const startStream = useCallback(
    async (op: () => Promise<void>) => {
      setLoading(true);
      setResult("");
      setMode(resolvedMode ?? "chat");
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
          setResult(`Error: ${event.payload.message}`);
          setLoading(false);
        });
        unlistenRef.current = [u1, u2, u3];
      } catch (e) {
        setResult(`Error: ${String(e)}`);
        setLoading(false);
      }
    },
    [cleanupListeners, resolvedMode],
  );

  // ── Command executors ─────────────────────────────────────────────
  const runChat = useCallback(
    (body: string) => startStream(async () => {
      const cid = await ensureConversation();
      await invoke("send_message", { conversationId: cid, content: body, providerId: activeProviderId, modelId: activeModelId });
    }),
    [startStream, ensureConversation, activeProviderId, activeModelId],
  );

  const runAgent = useCallback(
    (body: string) => startStream(async () => {
      const cid = await ensureConversation();
      await invoke("agent_query", { request: { conversationId: cid, input: body, providerId: activeProviderId, model_id: activeModelId } }, 0);
    }),
    [startStream, ensureConversation, activeProviderId, activeModelId],
  );

  const runUrl = useCallback(
    (url: string) => startStream(async () => {
      const cid = await ensureConversation();
      await invoke("agent_query", { request: { conversationId: cid, input: `Fetch the content from this URL and summarize it concisely (in 2-3 sentences max): ${url}`, providerId: activeProviderId, model_id: activeModelId } }, 0);
    }),
    [startStream, ensureConversation, activeProviderId, activeModelId],
  );

  const runSearch = useCallback(
    (body: string) => startStream(async () => {
      const results = await invoke<Array<{ content: string; score: number; title: string }>>("search_knowledge_base", { query: body, limit: 5 });
      if (!results || results.length === 0) { setResult("未找到相关知识"); setLoading(false); return; }
      const text = results.map((r) => `**${r.title}** (相关度: ${(r.score * 100).toFixed(0)}%)\n\n${r.content}`).join("\n\n---\n\n");
      setResult(text);
      setLoading(false);
    }),
    [startStream],
  );

  const runWiki = useCallback(
    async (body: string) => {
      if (!body.trim()) return;
      setLoading(true);
      setResult("");
      try {
        await invoke("llm_wiki_ingest", { title: `QuickBar - ${new Date().toLocaleString()}`, content: body });
        setResult("已存入知识库");
      } catch (e) { setResult(`存入失败: ${String(e)}`); }
      setLoading(false);
    },
    [],
  );

  const runCalc = useCallback(
    async (expr: string) => {
      try {
        const sanitized = expr.replace(/[^0-9+\-*/().%\s]/g, "");
        const value = Function(`"use strict"; return (${sanitized})`)();
        if (value === Infinity || value === -Infinity) throw new Error("Division by zero");
        setResult(`${expr.trim()} = ${Number.isInteger(value) ? value : value.toFixed(6)}`);
      } catch (e) {
        const cid = await ensureConversation();
        await startStream(async () => {
          await invoke("send_message", { conversationId: cid, content: `Calculate: ${expr}`, providerId: activeProviderId, modelId: activeModelId });
        });
        return;
      }
    },
    [startStream, ensureConversation, activeProviderId, activeModelId],
  );


  const runSwitchModel = useCallback((modelId: string) => {
    const settingsStore = useSettingsStore.getState();
    settingsStore.saveSettings({ default_model_id: modelId });
    setShowModelList(false);
    setResult(`模型已切换`);
    setTimeout(() => setResult(""), 1500);
  }, []);

  // ── Submit handler ─────────────────────────────────────────────────
  const handleSubmit = useCallback(async () => {
    const { command, body } = resolveCommand(input);
    if (!body) return;

    setShowCommands(false);
    pushRecent(input.trim());
    setRecentItems(loadRecent());

    switch (command) {
      case "chat": await runChat(body); break;
      case "agent": await runAgent(body); break;
      case "url": await runUrl(body); break;
      case "search": await runSearch(body); break;
      case "wiki": await runWiki(body); break;
      case "calc": await runCalc(body); break;
      case "model": setShowModelList(true); break;
    }
  }, [input, runChat, runAgent, runUrl, runSearch, runWiki, runCalc]);

  // Keyboard in command palette
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (showCommands) {
        if (e.key === "ArrowDown") { e.preventDefault(); setSelectedCmd((i) => Math.min(i + 1, COMMANDS.length - 1)); }
        else if (e.key === "ArrowUp") { e.preventDefault(); setSelectedCmd((i) => Math.max(i - 1, 0)); }
        else if (e.key === "Enter") {
          e.preventDefault();
          const cmd = COMMANDS[selectedCmd];
          setInput(`/${cmd.key} `);
          setShowCommands(false);
          setTimeout(() => inputRef.current?.focus(), 50);
        }
      }
    },
    [showCommands, selectedCmd],
  );

  const handleSelectCommand = useCallback((cmd: CommandDef) => {
    setInput(`/${cmd.key} `);
    setShowCommands(false);
    setTimeout(() => inputRef.current?.focus(), 50);
  }, []);

  // ── Copy ───────────────────────────────────────────────────────────
  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(result);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch { /* noop */ }
  }, [result]);

  const handleContinue = useCallback(() => {
    setInput((prev) => prev + "\n\n---\n" + result.slice(-500));
    setResult("");
    setTimeout(() => inputRef.current?.focus(), 50);
  }, [result]);

  const handleClear = useCallback(() => {
    setInput("");
    setResult("");
    setMode(null);
  }, []);

  const handleModelIconClick = useCallback(() => {
    setShowModelList((v) => !v);
  }, []);

  // ── UI short-hands ─────────────────────────────────────────────────
  const borderColor = token.colorBorderSecondary;

  const hasResult = result.length > 0;
  const showClear = input.length > 0 || hasResult;

  return (
    <div
      className="ax-page-transition"
      style={{ height: "100vh", display: "flex", flexDirection: "column", backgroundColor: token.colorBgContainer }}
    >
      {/* ── Header ─────────────────────────────────────────────────── */}
      <div
        className="ax-titlebar-compact title-bar-drag"
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          paddingLeft: 12,
          paddingRight: 8,
          height: 28,
          flexShrink: 0,
          borderBottom: `1px solid ${borderColor}`,
        }}
      >
        <span style={{ fontSize: 12, fontWeight: 500, color: token.colorTextSecondary }}>快捷入口</span>
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
          {activeModelId && (
            <Tooltip title={activeModelId}>
              <button
                className="title-bar-nodrag"
                onClick={handleModelIconClick}
                style={{ background: "none", border: "none", cursor: "pointer", padding: 2 }}
              >
                <ModelIcon model={activeModelId} size={16} type="avatar" />
              </button>
            </Tooltip>
          )}
          <button
            className="title-bar-nodrag"
            onClick={handleHide}
            style={{
              background: "none", border: "none", cursor: "pointer", padding: 4,
              color: token.colorTextSecondary, opacity: 0.5,
            }}
          >
            <X size={14} />
          </button>
        </div>
      </div>

      {/* ── Recent ──────────────────────────────────────────────────── */}
      {!hasResult && recentItems.length > 0 && (
        <div
          style={{
            display: "flex", alignItems: "center", gap: 8, padding: "4px 12px",
            fontSize: 11, color: token.colorTextQuaternary, flexShrink: 0,
            overflowX: "auto", whiteSpace: "nowrap",
          }}
        >
          <span>最近:</span>
          {recentItems.map((item, i) => (
            <span
              key={i}
              onClick={() => { setInput(item); setTimeout(() => inputRef.current?.focus(), 50); }}
              style={{ cursor: "pointer", opacity: 0.7, maxWidth: 160, overflow: "hidden", textOverflow: "ellipsis" }}
              title={item}
            >
              {item.length > 30 ? item.slice(0, 30) + "…" : item}
            </span>
          ))}
          <span
            onClick={() => { saveRecent([]); setRecentItems([]); }}
            style={{ cursor: "pointer", opacity: 0.4, marginLeft: "auto", flexShrink: 0 }}
          >
            清空
          </span>
        </div>
      )}

      {/* ── Input bar ───────────────────────────────────────────────── */}
      <div
        style={{
          display: "flex", alignItems: "center", gap: 8, padding: "8px 10px", flexShrink: 0,
          position: "relative",
        }}
      >
        <span style={{ opacity: 0.4, fontSize: 14, flexShrink: 0, color: token.colorTextSecondary }}>
          ▸
        </span>
        <Input
          ref={inputRef}
          placeholder={showCommands ? "选择命令..." : "输入 / 查看命令，或直接提问..."}
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
            <Tooltip title="清空">
              <button
                onClick={handleClear}
                style={{ background: "none", border: "none", cursor: "pointer", padding: 4, opacity: 0.4 }}
              >
                <X size={14} color={token.colorTextSecondary} />
              </button>
            </Tooltip>
          )}
      </div>

      {/* ── Command palette ──────────────────────────────────────────── */}
      {showCommands && (
        <div
          style={{
            margin: "0 10px", padding: "6px 0",
            backgroundColor: token.colorBgElevated,
            border: `1px solid ${borderColor}`,
            borderRadius: 8,
            boxShadow: `0 4px 16px rgba(0,0,0,0.2)`,
            zIndex: 10,
            flexShrink: 0,
          }}
        >
          {COMMANDS.map((cmd, idx) => (
            <div
              key={cmd.key}
              onClick={() => handleSelectCommand(cmd)}
              onMouseEnter={() => setSelectedCmd(idx)}
              style={{
                display: "flex", alignItems: "center", gap: 10, padding: "5px 12px",
                cursor: "pointer", fontSize: 12,
                backgroundColor: idx === selectedCmd ? token.colorFillSecondary : "transparent",
                color: token.colorText,
                transition: "background-color 0.1s",
              }}
            >
              <span style={{ opacity: 0.6, display: "flex", alignItems: "center" }}>{cmd.icon}</span>
              <span style={{ fontWeight: 500, fontFamily: "var(--code-font-family, monospace)", minWidth: 55 }}>
                /{cmd.label}
              </span>
              <span style={{ opacity: 0.5 }}>{cmd.hint}</span>
            </div>
          ))}
        </div>
      )}

      {/* ── Model switcher ───────────────────────────────────────────── */}
      {showModelList && (
        <div
          style={{
            margin: "0 10px", padding: "6px 0",
            backgroundColor: token.colorBgElevated,
            border: `1px solid ${borderColor}`,
            borderRadius: 8,
            boxShadow: `0 4px 16px rgba(0,0,0,0.2)`,
            zIndex: 10,
            flexShrink: 0,
            maxHeight: 200,
            overflowY: "auto",
          }}
        >
          {currentModels.map((m) => (
            <div
              key={m.model_id}
              onClick={() => runSwitchModel(m.model_id)}
              style={{
                display: "flex", alignItems: "center", gap: 8, padding: "6px 12px",
                cursor: "pointer", fontSize: 12,
                backgroundColor: m.model_id === activeModelId ? token.colorFillSecondary : "transparent",
                color: token.colorText,
                transition: "background-color 0.1s",
              }}
            >
              <ModelIcon model={m.model_id} size={16} type="avatar" />
              <span>{m.model_id}</span>
            </div>
          ))}
        </div>
      )}

      {/* ── Command hint bar ─────────────────────────────────────────── */}
      {!hasResult && !showCommands && !showModelList && (
        <div
          style={{
            display: "flex", gap: 12, padding: "2px 10px 6px",
            fontSize: 10, color: token.colorTextQuaternary, flexShrink: 0,
            overflow: "hidden", flexWrap: "wrap",
          }}
        >
          {COMMANDS.map((c) => (
            <span
              key={c.key}
              onClick={() => handleSelectCommand(c)}
              style={{ cursor: "pointer", whiteSpace: "nowrap" }}
            >
              /{c.label}
            </span>
          ))}
        </div>
      )}

      {/* ── Divider ──────────────────────────────────────────────────── */}
      {hasResult && (
        <div style={{ height: 1, backgroundColor: borderColor, flexShrink: 0 }} />
      )}

      {/* ── Result area ──────────────────────────────────────────────── */}
      {hasResult && (
        <>
          <div
            ref={resultRef}
            data-os-scrollbar
            style={{
              flex: 1, padding: "10px 14px", overflowY: "auto",
              fontSize: 13, lineHeight: 1.7, color: token.colorText,
              maxHeight: "60vh", whiteSpace: "pre-wrap", wordBreak: "break-word",
            }}
          >
            <Typography.Text style={{ fontSize: 13 }}>{result}</Typography.Text>
          </div>
          {/* ── Action bar ──────────────────────────────────────────── */}
          <div
            style={{
              display: "flex", alignItems: "center", gap: 6,
              padding: "6px 12px", borderTop: `1px solid ${borderColor}`,
              flexShrink: 0,
            }}
          >
            <Tooltip title={copied ? "已复制" : "复制结果"}>
              <button
                onClick={handleCopy}
                style={{
                  display: "inline-flex", alignItems: "center", gap: 4,
                  background: "none", border: "none", cursor: "pointer",
                  padding: "3px 8px", borderRadius: 4, fontSize: 11,
                  color: copied ? token.colorSuccess : token.colorTextSecondary,
                  transition: "background-color 0.15s",
                }}
                onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = token.colorFillSecondary; }}
                onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
              >
                <Copy size={12} /> {copied ? "已复制" : "复制"}
              </button>
            </Tooltip>
            <Tooltip title="存入知识库">
              <button
                onClick={async () => {
                  if (!result.trim()) return;
                  setLoading(true);
                  try {
                    await invoke("llm_wiki_ingest", { title: `QuickBar - ${new Date().toLocaleString()}`, content: result });
                    setResult((prev) => prev + "\n\n[已存入知识库]");
                  } catch (e) { setResult((prev) => prev + `\n\n[存入失败: ${String(e)}]`); }
                  setLoading(false);
                }}
                style={{
                  display: "inline-flex", alignItems: "center", gap: 4,
                  background: "none", border: "none", cursor: "pointer",
                  padding: "3px 8px", borderRadius: 4, fontSize: 11,
                  color: token.colorTextSecondary,
                  transition: "background-color 0.15s",
                }}
                onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = token.colorFillSecondary; }}
                onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
              >
                <BookOpen size={12} /> 存为知识
              </button>
            </Tooltip>
            <Tooltip title="将结果追加到输入继续追问">
              <button
                onClick={handleContinue}
                style={{
                  display: "inline-flex", alignItems: "center", gap: 4,
                  background: "none", border: "none", cursor: "pointer",
                  padding: "3px 8px", borderRadius: 4, fontSize: 11,
                  color: token.colorTextSecondary,
                  transition: "background-color 0.15s",
                }}
                onMouseEnter={(e) => { e.currentTarget.style.backgroundColor = token.colorFillSecondary; }}
                onMouseLeave={(e) => { e.currentTarget.style.backgroundColor = "transparent"; }}
              >
                <ArrowRight size={12} /> 继续追问
              </button>
            </Tooltip>
            <div style={{ flex: 1 }} />
            {mode && (
              <span style={{ fontSize: 10, color: token.colorTextQuaternary }}>
                /{mode}
              </span>
            )}
          </div>
        </>
      )}

      {/* ── Empty state ──────────────────────────────────────────────── */}
      {!hasResult && !showCommands && !showModelList && (
        <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
          <div style={{ textAlign: "center", opacity: 0.2 }}>
            <div style={{ fontSize: 40, marginBottom: 8 }}>▸</div>
            <div style={{ fontSize: 11, color: token.colorTextSecondary }}>输入 / 发现命令</div>
          </div>
        </div>
      )}
    </div>
  );
}
