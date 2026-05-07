import { useSkillExtensionStore } from "@/stores";
import { SkillPageRenderer } from "./SkillPageRenderer";

export function SkillPanels() {
  const panels = useSkillExtensionStore((s) => s.panels);

  const headerPanels = panels.filter((p) => p.position === "Header");
  const footerPanels = panels.filter((p) => p.position === "Footer");
  const mainPanels = panels.filter((p) => p.position === "Main");
  const sidebarPanels = panels.filter((p) => p.position === "Sidebar");

  if (panels.length === 0) { return null; }

  const renderPanel = (panel: typeof panels[number]) => (
    <div key={`${panel.skillName}:${panel.id}`}>
      <SkillPageRenderer
        componentType={panel.componentType}
        componentConfig={panel.componentConfig}
        skillName={panel.skillName}
      />
    </div>
  );

  return (
    <>
      {/* Header 面板 */}
      {headerPanels.length > 0 && (
        <div style={{ flexShrink: 0 }}>
          {headerPanels.map(renderPanel)}
        </div>
      )}

      {/* Main + Sidebar 布局 */}
      {(mainPanels.length > 0 || sidebarPanels.length > 0) && (
        <div style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
          {/* Main 面板 */}
          {mainPanels.length > 0 && (
            <div style={{ flex: 1, minWidth: 0, overflow: "auto" }}>
              {mainPanels.map(renderPanel)}
            </div>
          )}
          {/* Sidebar 面板 */}
          {sidebarPanels.length > 0 && (
            <div
              style={{
                width: 320,
                flexShrink: 0,
                overflow: "auto",
                borderLeft: "1px solid var(--color-border-secondary)",
              }}
            >
              {sidebarPanels.map(renderPanel)}
            </div>
          )}
        </div>
      )}

      {/* Footer 面板 */}
      {footerPanels.length > 0 && (
        <div style={{ flexShrink: 0 }}>
          {footerPanels.map(renderPanel)}
        </div>
      )}
    </>
  );
}
