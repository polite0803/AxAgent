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
      {
        key: "workflow",
        icon: <Workflow size={16} />,
        title: t("help.workflow", "Workflow"),
        items: [
          {
            key: "create",
            question: t("help.workflowCreateQ", "如何创建工作流？"),
            answer: t("help.workflowCreateA", "进入 Workflow 页面，从侧边栏拖拽节点到画布上，连接节点构建工作流。"),
          },
          {
            key: "run",
            question: t("help.workflowRunQ", "如何运行工作流？"),
            answer: t("help.workflowRunA", "点击工作流编辑器右上角的运行按钮，也可以逐步调试。"),
          },
        ],
      },
      {
        key: "skills",
        icon: <Puzzle size={16} />,
        title: t("help.skills", "Skills"),
        items: [
          {
            key: "install",
            question: t("help.skillsInstallQ", "如何安装技能？"),
            answer: t("help.skillsInstallA", "进入 Skills 页面，浏览并安装市场中的技能，或创建自定义技能。"),
          },
          {
            key: "use",
            question: t("help.skillsUseQ", "如何使用技能？"),
            answer: t("help.skillsUseA", "已安装的技能以斜杠命令形式出现在聊天输入框中，输入 / 查看可用技能。"),
          },
        ],
      },
      {
        key: "gateway",
        icon: <Globe size={16} />,
        title: t("help.gateway", "Gateway"),
        items: [
          {
            key: "setup",
            question: t("help.gatewaySetupQ", "如何设置网关？"),
            answer: t("help.gatewaySetupA", "进入 Gateway 页面，配置 API Key 和路由规则。网关允许统一访问多个供应商。"),
          },
          {
            key: "monitor",
            question: t("help.gatewayMonitorQ", "如何监控网关使用情况？"),
            answer: t("help.gatewayMonitorA", "Gateway 页面显示实时指标，包括请求次数、延迟和 Token 用量。"),
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
            aria-label={t("help.search", "搜索帮助主题…")}
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
                <span className="help-section__count">{section.items.length}</span>
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
