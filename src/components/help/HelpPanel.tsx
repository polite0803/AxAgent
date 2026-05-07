// 全局帮助面板 — ? 快捷键打开，右侧 Drawer
import { useHelpStore } from "@/stores/feature/helpStore";
import { Bot, Globe, Keyboard, MessageSquare, Search, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import "./HelpPanel.css";

interface HelpSection {
  key: string;
  icon: React.ReactNode;
  title: string;
  items: HelpItem[];
}

interface HelpItem {
  key: string;
  question: string;
  answer: string;
}

export function HelpPanel() {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [activeLocal, setActiveLocal] = useState<string | null>(null);
  const open = useHelpStore((s) => s.open);
  const toggleHelp = useHelpStore((s) => s.toggle);
  const closeHelp = useHelpStore((s) => s.close);
  const helpActiveSection = useHelpStore((s) => s.activeSection);

  // 外部打开指定 section 时自动展开
  useEffect(() => {
    if (helpActiveSection) { setActiveLocal(helpActiveSection); }
  }, [helpActiveSection]);

  const activeSection = activeLocal;

  const sections: HelpSection[] = useMemo(
    () => [
      {
        key: "chat",
        icon: <MessageSquare size={16} />,
        title: t("help.chat", "对话功能"),
        items: [
          {
            key: "send",
            question: t("help.chatSendQ", "如何发送消息？"),
            answer: t("help.chatSendA", "在底部输入框输入内容，按 Enter 发送。Shift+Enter 换行。"),
          },
          {
            key: "model",
            question: t("help.chatModelQ", "如何切换模型？"),
            answer: t("help.chatModelA", "点击输入框上方的模型选择器来切换。你可以在设置中管理模型供应商。"),
          },
          {
            key: "context",
            question: t("help.chatContextQ", "对话上下文如何工作？"),
            answer: t("help.chatContextA", "每次对话保持独立上下文。长对话会自动压缩以节省 Token。"),
          },
        ],
      },
      {
        key: "agent",
        icon: <Bot size={16} />,
        title: t("help.agent", "Agent 模式"),
        items: [
          {
            key: "switch",
            question: t("help.agentSwitchQ", "如何切换到 Agent 模式？"),
            answer: t("help.agentSwitchA", "在对话顶部的模式选择器中切换到 Agent 模式。"),
          },
          {
            key: "tool",
            question: t("help.agentToolQ", "Agent 可以使用哪些工具？"),
            answer: t(
              "help.agentToolA",
              "Agent 可以读写文件、执行 Bash 命令、搜索网络、调用 MCP 工具等。你可以在权限请求时选择批准或拒绝。",
            ),
          },
        ],
      },
      {
        key: "knowledge",
        icon: <Search size={16} />,
        title: t("help.knowledge", "知识库"),
        items: [
          {
            key: "upload",
            question: t("help.knowledgeUploadQ", "如何上传文档？"),
            answer: t("help.knowledgeUploadA", "在知识库页面拖拽或选择文件上传。支持 PDF、Markdown、纯文本等格式。"),
          },
        ],
      },
      {
        key: "shortcuts",
        icon: <Keyboard size={16} />,
        title: t("help.shortcuts", "快捷键"),
        items: [
          {
            key: "global",
            question: t("help.shortcutsGlobalQ", "有哪些常用快捷键？"),
            answer: t(
              "help.shortcutsGlobalA",
              "Ctrl+N: 新建对话 · Ctrl+Shift+I: 打开设置 · Ctrl+Shift+G: 打开网关 · ?: 打开帮助面板",
            ),
          },
        ],
      },
      {
        key: "providers",
        icon: <Globe size={16} />,
        title: t("help.providers", "模型供应商"),
        items: [
          {
            key: "add",
            question: t("help.providersAddQ", "如何添加模型供应商？"),
            answer: t("help.providersAddA", "在设置 → 模型供应商页面，点击添加按钮，选择供应商类型并填入配置。"),
          },
        ],
      },
    ],
    [t],
  );

  // 打开时阻止 body 滚动
  useEffect(() => {
    if (open) {
      const prev = document.body.style.overflow;
      document.body.style.overflow = "hidden";
      return () => {
        document.body.style.overflow = prev;
      };
    }
  }, [open]);

  // ? 快捷键
  useEffect(() => {
    const handleKeyDown = (e: globalThis.KeyboardEvent) => {
      if (
        e.key === "?"
        && !e.ctrlKey
        && !e.metaKey
        && !e.altKey
        && document.activeElement?.tagName !== "INPUT"
        && document.activeElement?.tagName !== "TEXTAREA"
      ) {
        e.preventDefault();
        toggleHelp();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const filtered = useMemo(() => {
    if (!search.trim()) { return sections; }
    const q = search.toLowerCase();
    return sections
      .map((s) => ({
        ...s,
        items: s.items.filter(
          (i) =>
            i.question.toLowerCase().includes(q)
            || i.answer.toLowerCase().includes(q),
        ),
      }))
      .filter((s) => s.items.length > 0);
  }, [search, sections]);

  if (!open) { return null; }

  return (
    <div className="help-panel" role="dialog" aria-label={t("help.title", "帮助中心")}>
      {/* 遮罩 */}
      <div className="help-panel__backdrop" onClick={closeHelp} />

      {/* 面板 */}
      <div className="help-panel__drawer">
        <div className="help-panel__header">
          <h3>{t("help.title", "帮助中心")}</h3>
          <button type="button" onClick={closeHelp} className="help-panel__close">
            <X size={16} />
          </button>
        </div>

        {/* 搜索框 */}
        <div className="help-panel__search">
          <Search size={14} />
          <input
            type="text"
            placeholder={t("help.search", "搜索帮助主题...") ?? undefined}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="help-panel__search-input"
          />
        </div>

        {/* 内容 */}
        <div className="help-panel__content">
          {filtered.map((section) => (
            <div key={section.key} className="help-section">
              <button
                type="button"
                className="help-section__toggle"
                onClick={() =>
                  setActiveLocal(
                    activeSection === section.key ? null : section.key,
                  )}
              >
                <span className="help-section__icon">{section.icon}</span>
                <span className="help-section__title">{section.title}</span>
                <span className="help-section__count">{section.items.length}</span>
              </button>
              {(activeSection === section.key || search.trim() !== "") && (
                <div className="help-section__items">
                  {section.items.map((item) => (
                    <div key={item.key} className="help-item">
                      <div className="help-item__q">{item.question}</div>
                      <div className="help-item__a">{item.answer}</div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
          {filtered.length === 0 && (
            <div className="help-panel__empty">
              {t("help.noResults", "未找到相关帮助")}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
