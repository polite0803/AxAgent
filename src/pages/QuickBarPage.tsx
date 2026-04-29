import { invoke, isTauri, listen, type UnlistenFn } from "@/lib/invoke";
import { useSettingsStore, useConversationStore } from "@/stores";
import type { Conversation, Message } from "@/types";
import { Input, theme, Typography, Tooltip } from "antd";
import {
  Camera,
  Globe,
  Loader2,
  Send,
  X,
  ArrowRight,
  FileText,
  BookOpen,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const QUICKBAR_CONV_KEY = "axagent_quickbar_conv_id";

function isUrl(text: string): boolean {
  return /^https?:\/\/\S+$/i.test(text.trim());
}

export function QuickBarPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState("");
  const [convId, setConvId] = useState<string | null>(
    () => localStorage.getItem(QUICKBAR_CONV_KEY),
  );
  const [mode, setMode] = useState<"qa" | "agent" | "url" | "wiki">("qa");
  const inputRef = useRef<any>(null);
  const unlistenRef = useRef<UnlistenFn[]>([]);

  const settings = useSettingsStore((s) => s.settings);
  const activeProviderId = settings.active_provider_id;
  const activeModelId = settings.active_model_id;

  // Auto-detect mode from input
  useEffect(() => {
    if (isUrl(input.trim())) {
      setMode("url");
    } else if (input.trim().startsWith(">")) {
      setMode("agent");
    } else {
      setMode("qa");
    }
  }, [input]);

  // Focus input on mount
  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 100);
  }, []);

  // Ensure conversation exists
  const ensureConversation = useCallback(async (): Promise<string> => {
    if (convId) return convId;

    const conversation = await invoke<Conversation>("create_conversation", {
      title: "QuickBar",
    });
    setConvId(conversation.id);
    localStorage.setItem(QUICKBAR_CONV_KEY, conversation.id);
    return conversation.id;
  }, [convId]);

  // Cleanup stream listeners
  const cleanupListeners = useCallback(() => {
    for (const fn of unlistenRef.current) {
      fn();
    }
    unlistenRef.current = [];
  }, []);

  useEffect(() => {
    return () => cleanupListeners();
  }, [cleanupListeners]);

  // Hide window
  const handleHide = useCallback(async () => {
    if (isTauri()) {
      await invoke("hide_quickbar");
    }
  }, []);

  // Handle Escape key
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        handleHide();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [handleHide]);

  // Handle URL mode: fetch content
  const handleUrlMode = useCallback(
    async (url: string) => {
      setLoading(true);
      setResult("");
      try {
        const cid = await ensureConversation();
        // Use agent_query to fetch the URL content
        await invoke(
          "agent_query",
          {
            request: {
              conversationId: cid,
              input: `Fetch the content from this URL and summarize it concisely (in 2-3 sentences max): ${url}`,
              providerId: activeProviderId,
              model_id: activeModelId,
            },
          },
          0,
        );

        let text = "";
        cleanupListeners();
        const unlistenText = await listen<{
          conversationId: string;
          assistantMessageId: string;
          text: string;
        }>("agent-stream-text", (event) => {
          text += event.payload.text;
          setResult(text);
        });
        const unlistenDone = await listen("agent-done", () => {
          setLoading(false);
        });
        const unlistenError = await listen<{ message: string }>(
          "agent-error",
          (event) => {
            setResult(`Error: ${event.payload.message}`);
            setLoading(false);
          },
        );
        unlistenRef.current = [unlistenText, unlistenDone, unlistenError];
      } catch (e) {
        setResult(`Error: ${String(e)}`);
        setLoading(false);
      }
    },
    [ensureConversation, activeProviderId, activeModelId, cleanupListeners],
  );

  // Handle Q&A mode
  const handleQaMode = useCallback(
    async (question: string) => {
      setLoading(true);
      setResult("");
      try {
        const cid = await ensureConversation();

        await invoke("send_message", {
          conversationId: cid,
          content: question,
          providerId: activeProviderId,
          modelId: activeModelId,
        });

        let text = "";
        cleanupListeners();
        const unlistenText = await listen<{
          conversationId: string;
          assistantMessageId: string;
          text: string;
        }>("agent-stream-text", (event) => {
          text += event.payload.text;
          setResult(text);
        });
        const unlistenDone = await listen("agent-done", () => {
          setLoading(false);
        });
        const unlistenError = await listen<{ message: string }>(
          "agent-error",
          (event) => {
            setResult(`Error: ${event.payload.message}`);
            setLoading(false);
          },
        );
        unlistenRef.current = [unlistenText, unlistenDone, unlistenError];
      } catch (e) {
        setResult(`Error: ${String(e)}`);
        setLoading(false);
      }
    },
    [ensureConversation, activeProviderId, activeModelId, cleanupListeners],
  );

  // Handle Agent mode (input starts with ">")
  const handleAgentMode = useCallback(
    async (task: string) => {
      const actualTask = task.startsWith(">") ? task.slice(1).trim() : task;
      if (!actualTask) return;

      setLoading(true);
      setResult("");
      try {
        const cid = await ensureConversation();

        await invoke(
          "agent_query",
          {
            request: {
              conversationId: cid,
              input: actualTask,
              providerId: activeProviderId,
              model_id: activeModelId,
            },
          },
          0,
        );

        let text = "";
        cleanupListeners();
        const unlistenText = await listen<{
          conversationId: string;
          assistantMessageId: string;
          text: string;
        }>("agent-stream-text", (event) => {
          text += event.payload.text;
          setResult(text);
        });
        const unlistenDone = await listen("agent-done", () => {
          setLoading(false);
        });
        const unlistenError = await listen<{ message: string }>(
          "agent-error",
          (event) => {
            setResult(`Error: ${event.payload.message}`);
            setLoading(false);
          },
        );
        unlistenRef.current = [unlistenText, unlistenDone, unlistenError];
      } catch (e) {
        setResult(`Error: ${String(e)}`);
        setLoading(false);
      }
    },
    [ensureConversation, activeProviderId, activeModelId, cleanupListeners],
  );

  // Handle submit
  const handleSubmit = useCallback(async () => {
    const trimmed = input.trim();
    if (!trimmed || loading) return;

    if (isUrl(trimmed)) {
      await handleUrlMode(trimmed);
    } else if (trimmed.startsWith(">")) {
      await handleAgentMode(trimmed);
    } else {
      await handleQaMode(trimmed);
    }
  }, [input, loading, handleUrlMode, handleAgentMode, handleQaMode]);

  // Handle screenshot
  const handleScreenshot = useCallback(async () => {
    setLoading(true);
    setResult("");
    try {
      const result = await invoke<{
        image_base64: string;
        width: number;
        height: number;
      }>("screen_capture", { monitor: 0 });
      setResult(`Screenshot captured: ${result.width}x${result.height}`);
      // Could pipe to vision analysis or save to wiki
    } catch (e) {
      setResult(`Screenshot error: ${String(e)}`);
    }
    setLoading(false);
  }, []);

  // Handle wiki save
  const handleSaveToWiki = useCallback(async () => {
    if (!result.trim()) return;
    try {
      await invoke("llm_wiki_ingest", {
        title: `QuickBar - ${new Date().toLocaleString()}`,
        content: result,
      });
      setResult((prev) => prev + "\n\n[Saved to Wiki]");
    } catch (e) {
      setResult((prev) => prev + `\n\n[Save failed: ${String(e)}]`);
    }
  }, [result]);

  const showClear = input.length > 0 || result.length > 0;

  return (
    <div
      className="flex flex-col"
      style={{
        height: "100vh",
        backgroundColor: token.colorBgContainer,
      }}
    >
      {/* Main input bar */}
      <div
        className="flex items-center gap-2"
        style={{
          padding: "6px 10px",
          borderBottom: result ? "1px solid var(--border-color)" : "none",
        }}
      >
        {/* Mode indicator */}
        <Tooltip
          title={
            mode === "url"
              ? "URL Fetch"
              : mode === "agent"
              ? "Agent Task"
              : "Quick Q&A"
          }
        >
          <span style={{ fontSize: 14, opacity: 0.5, flexShrink: 0 }}>
            {mode === "url" ? (
              <Globe size={16} />
            ) : mode === "agent" ? (
              <ArrowRight size={16} />
            ) : (
              <Send size={16} />
            )}
          </span>
        </Tooltip>

        <Input
          ref={inputRef}
          placeholder={
            mode === "url"
              ? "Paste URL to fetch..."
              : mode === "agent"
              ? "> Describe agent task..."
              : "Ask anything or paste URL..."
          }
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onPressEnter={handleSubmit}
          variant="borderless"
          disabled={loading}
          style={{ flex: 1, fontSize: 14 }}
        />

        {/* Action buttons */}
        <Tooltip title="Screenshot">
          <button
            onClick={handleScreenshot}
            disabled={loading}
            style={{
              background: "none",
              border: "none",
              cursor: loading ? "not-allowed" : "pointer",
              padding: 4,
              opacity: loading ? 0.3 : 0.6,
            }}
          >
            <Camera size={16} />
          </button>
        </Tooltip>

        {result && (
          <Tooltip title="Save to Wiki">
            <button
              onClick={handleSaveToWiki}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 4,
                opacity: 0.6,
              }}
            >
              <BookOpen size={16} />
            </button>
          </Tooltip>
        )}

        {loading ? (
          <Loader2 size={16} className="animate-spin" style={{ opacity: 0.6, flexShrink: 0 }} />
        ) : showClear ? (
          <Tooltip title="Clear (Esc to close)">
            <button
              onClick={() => {
                setInput("");
                setResult("");
                setMode("qa");
              }}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 4,
                opacity: 0.5,
              }}
            >
              <X size={16} />
            </button>
          </Tooltip>
        ) : null}
      </div>

      {/* Result area */}
      {result && (
        <div
          data-os-scrollbar
          style={{
            flex: 1,
            padding: "8px 14px",
            overflowY: "auto",
            fontSize: 13,
            lineHeight: 1.6,
            maxHeight: 300,
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
            color: token.colorText,
          }}
        >
          <Typography.Text style={{ fontSize: 13 }}>{result}</Typography.Text>
        </div>
      )}
    </div>
  );
}
