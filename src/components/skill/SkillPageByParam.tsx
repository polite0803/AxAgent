import { useSkillExtensionStore } from "@/stores";
import { Spin } from "antd";
import { useParams } from "react-router-dom";
import { SkillPageRenderer } from "./SkillPageRenderer";

function SkillPageByParam() {
  const { skillName, pageId } = useParams<{ skillName: string; pageId?: string }>();
  const pages = useSkillExtensionStore((s) => s.pages);

  if (!skillName) {
    return (
      <div style={{ padding: 24, textAlign: "center" }}>
        No skill specified.
      </div>
    );
  }

  // 通过 URL 参数匹配技能页面
  const page = pages.find((p) => {
    if (pageId) {
      return p.skillName === skillName && p.id === pageId;
    }
    return p.skillName === skillName;
  });

  if (!page) {
    return (
      <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>
        <Spin size="large" style={{ marginBottom: 16, display: "block" }} />
        Loading skill "{skillName}"...
      </div>
    );
  }

  return (
    <SkillPageRenderer
      componentType={page.componentType}
      componentConfig={page.componentConfig}
      skillName={page.skillName}
    />
  );
}

export default SkillPageByParam;
export { SkillPageByParam };
