import { useTranslation } from "react-i18next";
import { useWorkflowEditorStore } from "@/stores";
import { Input, Tabs, Tag } from "antd";
import { FileText, Search } from "lucide-react";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { NODE_CATEGORIES, NODE_TYPE_MAP } from "../types";
import { setDragPayload, type DragPayload } from "../dndState";

export const LeftPanel: React.FC = () => {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const { templates, loadTemplate } = useWorkflowEditorStore();
  const [isDragging, setIsDragging] = useState(false);
  const dragRef = useRef<DragPayload | null>(null);
  const ghostRef = useRef<HTMLDivElement | null>(null);

  const handleMouseDown = useCallback(
    (event: React.MouseEvent, nodeType: string, nodeLabel: string) => {
      if (event.button !== 0) return;

      event.preventDefault();

      const payload: DragPayload = { type: nodeType, label: nodeLabel };
      dragRef.current = payload;
      setDragPayload(payload);
      setIsDragging(true);

      // Create a ghost element that follows the cursor
      const ghost = document.createElement("div");
      ghost.textContent = nodeLabel;
      ghost.style.position = "fixed";
      ghost.style.pointerEvents = "none";
      ghost.style.zIndex = "99999";
      ghost.style.padding = "6px 12px";
      ghost.style.background = "#333";
      ghost.style.color = "#fff";
      ghost.style.borderRadius = "4px";
      ghost.style.fontSize = "12px";
      ghost.style.whiteSpace = "nowrap";
      ghost.style.opacity = "0.85";
      ghost.style.left = `${event.clientX + 12}px`;
      ghost.style.top = `${event.clientY + 12}px`;
      document.body.appendChild(ghost);
      ghostRef.current = ghost;
    },
    [],
  );

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (ghostRef.current) {
        ghostRef.current.style.left = `${e.clientX + 12}px`;
        ghostRef.current.style.top = `${e.clientY + 12}px`;
      }
    };

    const handleMouseUp = () => {
      setIsDragging(false);
      dragRef.current = null;
      if (ghostRef.current) {
        ghostRef.current.remove();
        ghostRef.current = null;
      }
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isDragging]);

  const filteredNodeTypes = Object.entries(NODE_TYPE_MAP)
    .filter(([_, info]) => info.label.toLowerCase().includes(search.toLowerCase()))
    .filter(([type, _]) => !type.startsWith("_"))
    .filter(([_, info]) => !info.label.includes(t("workflow.leftPanel.legacySuffix")));

  const groupedNodeTypes = NODE_CATEGORIES.map((category) => ({
    ...category,
    items: filteredNodeTypes.filter(([_, info]) => info.category === category.id),
  })).filter((category) => category.items.length > 0);

  const handleTemplateClick = (templateId: string) => {
    loadTemplate(templateId);
  };

  return (
    <div
      style={{
        width: 280,
        background: "#252525",
        borderRight: "1px solid #333",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <Tabs
        defaultActiveKey="nodes"
        size="small"
        style={{ height: "100%" }}
        items={[
          {
            key: "nodes",
            label: t("workflow.leftPanel.nodesTab"),
            children: (
              <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
                <Input
                  prefix={<Search size={14} style={{ color: "#666" }} />}
                   placeholder={t("workflow.leftPanel.searchNodes")}
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  style={{ margin: "8px", width: "auto" }}
                  size="small"
                />

                <div style={{ flex: 1, overflow: "auto", padding: "0 8px" }}>
                  {groupedNodeTypes.map((category) => (
                    <div key={category.id} style={{ marginBottom: 12 }}>
                      <div
                        style={{
                          fontSize: 11,
                          color: "#666",
                          textTransform: "uppercase",
                          marginBottom: 6,
                          paddingLeft: 4,
                        }}
                      >
                        {category.label}
                      </div>
                      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 6 }}>
                        {category.items.map(([type, info]) => (
                          <div
                            key={type}
                            onMouseDown={(e) => handleMouseDown(e, type, info.label)}
                            style={{
                              padding: "8px 6px",
                              background: "#1a1a1a",
                              border: `1px solid ${info.color}40`,
                              borderRadius: 6,
                              cursor: "grab",
                              textAlign: "center",
                              fontSize: 11,
                              color: "#ccc",
                              transition: "all 0.2s",
                              userSelect: "none",
                            }}
                            onMouseEnter={(e) => {
                              e.currentTarget.style.borderColor = info.color;
                              e.currentTarget.style.background = `${info.color}10`;
                            }}
                            onMouseLeave={(e) => {
                              e.currentTarget.style.borderColor = `${info.color}40`;
                              e.currentTarget.style.background = "#1a1a1a";
                            }}
                          >
                            <div style={{ fontSize: 16, marginBottom: 4 }}>{getNodeIcon(type)}</div>
                            <div style={{ whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                              {info.label}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ),
          },
          {
            key: "templates",
            label: t("workflow.leftPanel.templatesTab"),
            children: (
              <div style={{ padding: "8px" }}>
                <Input
                  prefix={<Search size={14} style={{ color: "#666" }} />}
                   placeholder={t("workflow.leftPanel.searchTemplates")}
                  style={{ marginBottom: 8 }}
                  size="small"
                />
                <div style={{ overflow: "auto", maxHeight: "calc(100vh - 200px)" }}>
                  {templates.map((template) => (
                    <div
                      key={template.id}
                      onClick={() => handleTemplateClick(template.id)}
                      style={{
                        padding: 10,
                        marginBottom: 6,
                        background: "#1a1a1a",
                        borderRadius: 6,
                        cursor: "pointer",
                        border: "1px solid transparent",
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.borderColor = "#1890ff40";
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.borderColor = "transparent";
                      }}
                    >
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <FileText size={14} style={{ color: "#1890ff" }} />
                        <span style={{ color: "#ccc", fontSize: 12 }}>{template.name}</span>
                        {template.is_preset && (
                          <Tag color="blue" style={{ fontSize: 10, margin: 0 }}>
                            {t("workflow.preset")}
                          </Tag>
                        )}
                      </div>
                      {template.description && (
                        <div
                          style={{
                            color: "#666",
                            fontSize: 11,
                            marginTop: 4,
                            marginLeft: 22,
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          }}
                        >
                          {template.description}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            ),
          },
        ]}
      />
    </div>
  );
};

function getNodeIcon(type: string): string {
  const icons: Record<string, string> = {
    trigger: "⚡",
    agent: "🤖",
    llm: "🧠",
    condition: "❓",
    parallel: "⏩",
    loop: "🔄",
    merge: "🔗",
    delay: "⏱",
    atomicSkill: "⚛️",
    tool: "🔧",
    code: "💻",
    subWorkflow: "📦",
    documentParser: "📄",
    vectorRetrieve: "🔍",
    end: "🏁",
    validation: "✓",
  };
  return icons[type] || "📦";
}
