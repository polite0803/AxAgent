// 全局帮助面板 — ? 快捷键打开，右侧 Drawer
import { useHelpStore } from "@/stores/feature/helpStore";
import { Bot, Globe, Keyboard, MessageSquare, Puzzle, Search, Workflow, X } from "lucide-react";
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
    if (helpActiveSection) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setActiveLocal(helpActiveSection);
    }
  }, [helpActiveSection]);

  const activeSection = activeLocal;

  const sections: HelpSection[] = useMemo(
    () => [
      {
        key: "chat",
        icon: <MessageSquare size={16} />,
        title: t("help.chat"),
        items: [
          {
            key: "send",
            question: t("help.chatSendQ"),
            answer: t("help.chatSendA"),
          },
          {
            key: "model",
            question: t("help.chatModelQ"),
            answer: t("help.chatModelA"),
          },
          {
            key: "context",
            question: t("help.chatContextQ"),
            answer: t("help.chatContextA"),
          },
        ],
      },
      {
        key: "agent",
        icon: <Bot size={16} />,
        title: t("help.agent"),
        items: [
          {
            key: "switch",
            question: t("help.agentSwitchQ"),
            answer: t("help.agentSwitchA"),
          },
          {
            key: "tool",
            question: t("help.agentToolQ"),
            answer: t("help.agentToolA"),
          },
        ],
      },
      {
        key: "knowledge",
        icon: <Search size={16} />,
        title: t("help.knowledge"),
        items: [
          {
            key: "upload",
            question: t("help.knowledgeUploadQ"),
            answer: t("help.knowledgeUploadA"),
          },
        ],
      },
      {
        key: "shortcuts",
        icon: <Keyboard size={16} />,
        title: t("help.shortcuts"),
        items: [
          {
            key: "global",
            question: t("help.shortcutsGlobalQ"),
            answer: t("help.shortcutsGlobalA"),
          },
        ],
      },
      {
        key: "providers",
        icon: <Globe size={16} />,
        title: t("help.providers"),
        items: [
          {
            key: "add",
            question: t("help.providersAddQ"),
            answer: t("help.providersAddA"),
          },
        ],
      },
      {
        key: "workflow",
        icon: <Workflow size={16} />,
        title: t("help.workflow"),
        items: [
          {
            key: "create",
            question: t("help.workflowCreateQ"),
            answer: t("help.workflowCreateA"),
          },
          {
            key: "run",
            question: t("help.workflowRunQ"),
            answer: t("help.workflowRunA"),
          },
        ],
      },
      {
        key: "skills",
        icon: <Puzzle size={16} />,
        title: t("help.skills"),
        items: [
          {
            key: "install",
            question: t("help.skillsInstallQ"),
            answer: t("help.skillsInstallA"),
          },
          {
            key: "use",
            question: t("help.skillsUseQ"),
            answer: t("help.skillsUseA"),
          },
        ],
      },
      {
        key: "gateway",
        icon: <Globe size={16} />,
        title: t("help.gateway"),
        items: [
          {
            key: "setup",
            question: t("help.gatewaySetupQ"),
            answer: t("help.gatewaySetupA"),
          },
          {
            key: "monitor",
            question: t("help.gatewayMonitorQ"),
            answer: t("help.gatewayMonitorA"),
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const filtered = useMemo(() => {
    if (!search.trim()) {
      return sections;
    }
    const q = search.toLowerCase();
    return sections.flatMap((s) => {
      const items = s.items.filter(
        (i) =>
          i.question.toLowerCase().includes(q)
          || i.answer.toLowerCase().includes(q),
      );
      return items.length > 0 ? [{ ...s, items }] : [];
    });
  }, [search, sections]);

  if (!open) {
    return null;
  }

  return (
    <div className="help-panel" role="dialog" aria-label={t("help.title")}>
      {/* 遮罩 */}
      <div
        className="help-panel__backdrop"
        role="presentation"
        onClick={closeHelp}
      />

      {/* 面板 */}
      <div className="help-panel__drawer">
        <div className="help-panel__header">
          <h3>{t("help.title")}</h3>
          <button
            type="button"
            onClick={closeHelp}
            className="help-panel__close"
          >
            <X size={16} />
          </button>
        </div>

        {/* 搜索框 */}
        <div className="help-panel__search">
          <Search size={14} />
          <input
            type="text"
            placeholder={t("help.search") ?? undefined}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="help-panel__search-input"
            aria-label={t("help.search")}
          />
        </div>

        {/* 内容 */}
        <div className="help-panel__content">
          {filtered.map((section) => (
            <div key={section.key} className="help-section">
              <button
                type="button"
                className="help-section__toggle"
                id={`help-section-toggle-${section.key}`}
                aria-expanded={activeSection === section.key || search.trim() !== ""}
                aria-controls={`help-section-${section.key}`}
                onClick={() =>
                  setActiveLocal(
                    activeSection === section.key ? null : section.key,
                  )}
              >
                <span className="help-section__icon">{section.icon}</span>
                <span className="help-section__title">{section.title}</span>
                <span className="help-section__count">
                  {section.items.length}
                </span>
              </button>
              {(activeSection === section.key || search.trim() !== "") && (
                <div
                  className="help-section__items"
                  id={`help-section-${section.key}`}
                  role="region"
                  aria-labelledby={`help-section-toggle-${section.key}`}
                >
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
          {filtered.length === 0 && <div className="help-panel__empty">{t("help.noResults")}</div>}
        </div>
      </div>
    </div>
  );
}
