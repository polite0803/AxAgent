import { useSkillExtensionStore } from "@/stores";
import { Button } from "antd";
import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import { SkillPageRenderer } from "./SkillPageRenderer";

/** 面板尺寸映射 */
const SIZE_MAP: Record<string, number> = {
  Small: 240,
  Medium: 320,
  Large: 480,
  FullWidth: -1,
};

export function SkillPanels() {
  const panels = useSkillExtensionStore((s) => s.panels);

  const headerPanels = panels.filter((p) => p.position === "Header");
  const footerPanels = panels.filter((p) => p.position === "Footer");
  const mainPanels = panels.filter((p) => p.position === "Main");
  const sidebarPanels = panels.filter((p) => p.position === "Sidebar");

  if (panels.length === 0) { return null; }

  const renderPanel = (panel: typeof panels[number]) => (
    <CollapsiblePanel key={`${panel.skillName}:${panel.id}`} panel={panel} />
  );

  return (
    <>
      {headerPanels.length > 0 && (
        <div style={{ flexShrink: 0 }}>
          {headerPanels.map(renderPanel)}
        </div>
      )}

      {(mainPanels.length > 0 || sidebarPanels.length > 0) && (
        <div style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
          {mainPanels.length > 0 && (
            <div style={{ flex: 1, minWidth: 0, overflow: "auto" }}>
              {mainPanels.map(renderPanel)}
            </div>
          )}
          {sidebarPanels.length > 0 && (
            <div
              style={{
                flexShrink: 0,
                overflow: "auto",
                borderLeft: "1px solid var(--color-border-secondary)",
              }}
            >
              {sidebarPanels.map((panel) => {
                const size = SIZE_MAP[panel.size] || 320;
                return (
                  <div key={`${panel.skillName}:${panel.id}`} style={{ width: size > 0 ? size : "100%" }}>
                    <CollapsiblePanel panel={panel} />
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {footerPanels.length > 0 && (
        <div style={{ flexShrink: 0 }}>
          {footerPanels.map(renderPanel)}
        </div>
      )}
    </>
  );
}

function CollapsiblePanel({ panel }: { panel: ReturnType<typeof useSkillExtensionStore.getState>["panels"][number] }) {
  const [collapsed, setCollapsed] = useState(panel.collapsible ? panel.defaultCollapsed : false);

  if (!panel.collapsible) {
    return (
      <SkillPageRenderer
        componentType={panel.componentType}
        componentConfig={panel.componentConfig}
        skillName={panel.skillName}
      />
    );
  }

  return (
    <div>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 4,
          padding: "4px 8px",
          cursor: "pointer",
          fontSize: 12,
          fontWeight: 500,
          color: "var(--color-text-secondary)",
          borderBottom: collapsed ? "none" : "1px solid var(--color-border-secondary)",
        }}
        onClick={() => setCollapsed(!collapsed)}
      >
        <Button type="text" size="small" icon={collapsed ? <ChevronRight size={12} /> : <ChevronDown size={12} />} />
        {panel.title}
      </div>
      {!collapsed && (
        <SkillPageRenderer
          componentType={panel.componentType}
          componentConfig={panel.componentConfig}
          skillName={panel.skillName}
        />
      )}
    </div>
  );
}
