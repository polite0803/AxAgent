import { useSkillExtensionStore } from "@/stores";
import { SkillPageRenderer } from "./SkillPageRenderer";

export function SkillPanels() {
  const panels = useSkillExtensionStore((s) => s.panels);

  const headerPanels = panels.filter((p) => p.position === "Header");
  const footerPanels = panels.filter((p) => p.position === "Footer");

  if (panels.length === 0) { return null; }

  return (
    <>
      {/* Header 面板 */}
      {headerPanels.length > 0 && (
        <div style={{ flexShrink: 0 }}>
          {headerPanels.map((panel) => (
            <div key={`${panel.skillName}:${panel.id}`}>
              <SkillPageRenderer
                componentType={panel.componentType}
                componentConfig={panel.componentConfig}
                skillName={panel.skillName}
              />
            </div>
          ))}
        </div>
      )}

      {/* Footer 面板 */}
      {footerPanels.length > 0 && (
        <div style={{ flexShrink: 0 }}>
          {footerPanels.map((panel) => (
            <div key={`${panel.skillName}:${panel.id}`}>
              <SkillPageRenderer
                componentType={panel.componentType}
                componentConfig={panel.componentConfig}
                skillName={panel.skillName}
              />
            </div>
          ))}
        </div>
      )}
    </>
  );
}
